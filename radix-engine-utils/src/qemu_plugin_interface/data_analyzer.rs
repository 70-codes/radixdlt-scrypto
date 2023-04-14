use fixedstr::{str32, str64};
use std::{
    fmt::{Display, Formatter},
    fs::File,
    io::Write,
};

#[derive(Clone)]
pub enum OutputDataEvent {
    FunctionEnter,
    FunctionExit,
}

impl Display for OutputDataEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            OutputDataEvent::FunctionEnter => f.write_fmt(format_args!("enter")).unwrap(),
            OutputDataEvent::FunctionExit => f.write_fmt(format_args!("exit")).unwrap(),
        };
        Ok(())
    }
}

#[derive(Clone)]
pub enum OutputParamValue {
    NumberI64(i64),
    NumberU64(u64),
    Literal(str64), // using contant 64-bytes length string for speed optimisation
}
impl Display for OutputParamValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            OutputParamValue::NumberI64(v) => f.write_fmt(format_args!("{}", v)).unwrap(),
            OutputParamValue::NumberU64(v) => f.write_fmt(format_args!("{}", v)).unwrap(),
            OutputParamValue::Literal(v) => f.write_fmt(format_args!("{}", v)).unwrap(),
        };
        Ok(())
    }
}
impl Default for OutputParamValue {
    fn default() -> Self {
        OutputParamValue::Literal(str64::new())
    }
}

#[derive(Clone)]
pub struct OutputParam {
    pub name: str32,
    pub value: OutputParamValue,
}
impl OutputParam {
    pub fn new(name: &str, value: OutputParamValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

#[derive(Clone)]
pub struct OutputData<'a> {
    /// Logged event
    pub event: OutputDataEvent,
    /// Current stack depth
    pub stack_depth: usize,
    /// CPU instructions count
    pub cpu_instructions: u64,
    /// CPU instructions count with subtracted calibration values
    pub cpu_instructions_calibrated: u64,
    /// Called function name
    pub function_name: &'a str,
    /// Function parameters to log
    pub param: Vec<OutputParam>,
}

impl<'a> OutputData<'a> {
    pub fn write(&self, file: &mut File) {
        let spaces = std::iter::repeat(' ')
            .take(4 * self.stack_depth)
            .collect::<String>();

        match self.event {
            OutputDataEvent::FunctionEnter => file
                .write_fmt(format_args!(
                    "{}++enter: {} {}",
                    spaces, self.function_name, self.stack_depth
                ))
                .expect("Unable to write output data"),
            OutputDataEvent::FunctionExit => file
                .write_fmt(format_args!(
                    "{}--exit: {} {} {} {}",
                    spaces,
                    self.function_name,
                    self.stack_depth,
                    self.cpu_instructions,
                    self.cpu_instructions_calibrated
                ))
                .expect("Unable to write output data"),
        };

        for p in &self.param {
            file.write_fmt(format_args!(
                " {}=\"{}\"",
                p.name,
                p.value.to_string().replace('\"', "&quot;")
            ))
            .expect(&format!("Unable write data."));
        }

        file.write_fmt(format_args!("\n"))
            .expect(&format!("Unable write data."));
    }

    pub fn is_return_from_function(&self) -> bool {
        for p in &self.param {
            if p.name == "return" {
                match p.value {
                    OutputParamValue::Literal(v) => {
                        if v == "true" {
                            return true;
                        }
                    }
                    _ => (),
                }
            }
        }
        false
    }
}

pub struct DataAnalyzer {}
impl DataAnalyzer {
    /// Function discards spikes in passed vector data, used for calibration.
    pub fn discard_spikes(data: &mut Vec<u64>, delta_range: u64) {
        // 1. calculate median
        data.sort();
        let center_idx = data.len() / 2;
        let median = data[center_idx];

        // 2. discard items out of median + range
        data.retain(|&i| {
            if i > median {
                i - median <= delta_range
            } else {
                median - i <= delta_range
            }
        });
    }

    /// Function calculates average for passed vector.
    pub fn average(data: &Vec<u64>) -> u64 {
        data.iter().sum::<u64>() / data.len() as u64
    }

    /// Function stores passed data as csv file.
    pub fn save_csv<'a>(data: &Vec<OutputData<'a>>, file_name: &str) {
        if let Ok(mut file) = File::create(file_name) {
            file.write_fmt(format_args!("event;function_name;stack_depth;instructions_count;instructions_count_calibrated\n")).expect(&format!("Unable write to {} file.", file_name));

            for v in data {
                file.write_fmt(format_args!(
                    "{};{};{};{};{}\n",
                    v.event,
                    v.function_name,
                    v.stack_depth,
                    v.cpu_instructions,
                    v.cpu_instructions_calibrated
                ))
                .expect(&format!("Unable write to {} file.", file_name));
            }
            file.flush()
                .expect(&format!("Unable to flush {} file.", file_name))
        } else {
            panic!("Unable to create {} file.", file_name)
        }
    }

    /// Function stores passed data as xml file.
    pub fn save_xml<'a>(data: &mut Vec<OutputData<'a>>, file_name: &str) {
        // ensure folder exists
        let mut path = std::path::PathBuf::new();
        path.push(file_name);
        path.pop();
        std::fs::create_dir_all(path).unwrap_or_default();

        if let Ok(mut file) = File::create(file_name) {
            let mut stack_fcn: Vec<&'a str> = vec!["root"];
            let mut prev_stack_depth = 0;
            file.write_fmt(format_args!("<root>\n")).unwrap();

            let mut data_to_insert: Vec<(usize, OutputData<'a>)> = Vec::new();

            for (i, v) in data.iter().enumerate() {
                // for each function enter event
                if matches!(v.event, OutputDataEvent::FunctionEnter) {
                    // verify function exit with same stack depth is present
                    let mut found = false;
                    let mut idx = 0;
                    for (j, w) in data[i+1..].into_iter().enumerate() {
                        if v.stack_depth == w.stack_depth
                            && v.function_name == w.function_name
                            && matches!(w.event, OutputDataEvent::FunctionExit)
                        {
                            found = true;
                            break;
                        } else if w.stack_depth < v.stack_depth ||
                            (w.stack_depth == v.stack_depth && v.function_name != w.function_name) {
                            // not found due to stack depth diff or function name diff
                            // exit event must be added before j element
                            idx = i + 1 + j;
                            break;
                        } else if w.stack_depth > v.stack_depth {
                            // ok
                            idx = i + 1 + j + 1; // update idx in case of stack depth 0 function missing
                        } else {
                            panic!("Wrong sequence of data: {}:{} (idx {}), {}:{} (idx {})", v.stack_depth, v.function_name, i, w.function_name, w.stack_depth, j)
                        }
                    }

                    if !found {
                        println!("======== Inserting: {} {} {}", v.function_name, v.stack_depth, idx);
                        data_to_insert.push( (idx, OutputData{
                            event: OutputDataEvent::FunctionExit,
                            stack_depth: v.stack_depth,
                            cpu_instructions: v.cpu_instructions,
                            cpu_instructions_calibrated: v.cpu_instructions_calibrated,
                            function_name: v.function_name,
                            param: Vec::new()
                        }) );
                    }
                }
            }

            for v in data_to_insert.iter().rev() {
                println!("======== Inserte1: {} {}", v.0, data.len());
                data.insert( v.0, v.1.clone() );
                println!("======== Inserte2: {} {}", v.0, data.len());
            }

            for (i, v) in data.iter().enumerate() {
                let mut cpu_ins_cal = v.cpu_instructions_calibrated;
                let mut param: &Vec<OutputParam> = &Vec::new();

                // get cpu instructions and param from exit event
                if matches!(v.event, OutputDataEvent::FunctionEnter) {
                    let mut found = false;
                    for w in data[i+1..].into_iter() {
                        if v.stack_depth == w.stack_depth
                            && v.function_name == w.function_name
                            && matches!(w.event, OutputDataEvent::FunctionExit)
                        {
                            cpu_ins_cal = w.cpu_instructions_calibrated;
                            param = &w.param;
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        println!("Function exit not found: {}:{} (idx {})", v.stack_depth, v.function_name, i)
                    }
                }

                if v.stack_depth > prev_stack_depth {
                    file.write_fmt(format_args!(">\n")).unwrap();
                } else if v.stack_depth < prev_stack_depth {
                    let spaces = std::iter::repeat(' ')
                        .take(v.stack_depth)
                        .collect::<String>();
                    file.write_fmt(format_args!("{}</{}>\n", spaces, stack_fcn.pop().unwrap()))
                        .unwrap();
                } else if i > 0 && matches!(v.event, OutputDataEvent::FunctionExit) {
                    file.write_fmt(format_args!("/>\n")).unwrap();
                    stack_fcn.pop();
                }

                if !matches!(v.event, OutputDataEvent::FunctionExit) {
                    let spaces = std::iter::repeat(' ')
                        .take(v.stack_depth)
                        .collect::<String>();
                    stack_fcn.push(v.function_name);

                    file.write_fmt(format_args!(
                        "{}<{} ins=\"{}\"",
                        spaces, v.function_name, cpu_ins_cal
                    ))
                    .expect(&format!("Unable write to {} file.", file_name));

                    if !param.is_empty() {
                        // use param from exit event if available
                        for p in param {
                            file.write_fmt(format_args!(
                                " {}=\"{}\"",
                                p.name,
                                p.value.to_string().replace('\"', "&quot;")
                            ))
                            .expect(&format!("Unable write to {} file.", file_name));
                        }
                    }
                    if !v.param.is_empty() {
                        for p in &v.param {
                            // skip same name argument, as they are prohibited in XML
                            if param
                                .into_iter()
                                .find(|&item| item.name == p.name)
                                .is_some()
                            {
                                continue;
                            }
                            file.write_fmt(format_args!(
                                " {}=\"{}\"",
                                p.name,
                                p.value.to_string().replace('\"', "&quot;")
                            ))
                            .expect(&format!("Unable write to {} file.", file_name));
                        }
                    }
                }

                prev_stack_depth = v.stack_depth;
            }
            file.write_fmt(format_args!("</root>")).unwrap();

            file.flush()
                .expect(&format!("Unable to flush {} file.", file_name))
        } else {
            panic!("Unable to create {} file.", file_name)
        }
    }
}
