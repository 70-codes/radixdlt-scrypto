#!/bin/bash

set -x
set -e

cd "$(dirname "$0")/.."

resim="cargo run --bin resim $@ --"

$resim reset

export account=`$resim new-account | awk '/Account component address:/ {print $NF}'`
export owner_badge=`$resim new-simple-badge --name 'OwnerBadge' | awk '/NFAddress:/ {print $NF}'`
export package=`$resim publish ../examples/hello-world $owner_badge | awk '/Package:/ {print $NF}'`

export xrd=resource_sim1qzxcrac59cy2v9lpcpmf82qel3cjj25v3k5m09rxurgqehgxzu

output=`$resim run ./tests/m1.rtm | awk '/Component:|Resource:/ {print $NF}'`
export component=`echo $output | cut -d " " -f1`
export resource=`echo $output | cut -d " " -f2`

$resim run ./tests/m2.rtm

$resim show-ledger