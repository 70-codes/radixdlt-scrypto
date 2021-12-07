#!/bin/bash

set -x
set -e

cd "$(dirname "$0")/.."

resim="cargo run --bin resim $@ --"

# Set up environment
$resim reset
temp=`$resim new-account | tee /dev/tty | awk '/Component:|Public key:/ {print $NF}'`
account=`echo $temp | cut -d " " -f1`
account_key=`echo $temp | cut -d " " -f2`
account2=`$resim new-account | tee /dev/tty | awk '/Component:/ {print $NF}'`
mint_badge=`$resim new-badge-fixed 1 --name 'MintBadge' | tee /dev/tty | awk '/ResourceDef:/ {print $NF}'`
resource_def=`$resim new-token-mutable $mint_badge | tee /dev/tty | awk '/ResourceDef:/ {print $NF}'`
$resim mint 777 $resource_def $mint_badge --signers $account_key
$resim transfer 111,$resource_def $account2 --signers $account_key

# Test hello-world
package=`$resim publish ../examples/core/hello-world | tee /dev/tty | awk '/Package:/ {print $NF}'`
component=`$resim call-function $package Hello new | tee /dev/tty | awk '/Component:/ {print $NF}'`
$resim call-method $component free_token

# Test cross component call
$resim publish ../examples/core/cross-blueprint-call --address 01bda8686d6c2fa45dce04fac71a09b54efbc8028c23aac74bc00e
package=`$resim publish ../examples/core/cross-blueprint-call | tee /dev/tty | awk '/Package:/ {print $NF}'`
component=`$resim call-function $package Proxy1 new | tee /dev/tty | awk '/Component:/ {print $NF}' | tail -n1`
$resim call-method $component free_token
component=`$resim call-function $package Proxy2 new | tee /dev/tty | awk '/Component:/ {print $NF}' | tail -n1`
$resim call-method $component free_token

# Export abi
$resim export-abi $package Proxy1

# Show state
$resim show $package
$resim show $component
$resim show $account
$resim show $account2