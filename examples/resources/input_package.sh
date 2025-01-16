#!/bin/bash

PACKAGES="bms-algorithm-performance"\
" bms-algorithm-eco"\
" bms-algorithm-charging"\
" antipinch-enable"\
" antipinch-disable"

for package in $PACKAGES
do
    curl --location 'http://0.0.0.0:47099/package' \
    --header 'Content-Type: text/plain' \
    --data "${package}"
done