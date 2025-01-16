#!/bin/bash

echo "1: eco, 2:performance, 3: anti-enable, 4: anti-disable :"
read input
A=$((input))
BODY=""

if [ "$A" -eq 1 ]; then
	BODY="bms/bms-eco-mode"
elif [ "$A" -eq 2 ]; then
	BODY="bms/bms-high-performance"
elif [ "$A" -eq 3 ]; then
        BODY="antipinch/antipinch-enable"
elif [ "$A" -eq 4 ]; then
        BODY="antipinch/antipinch-disable"
fi

curl --location 'http://0.0.0.0:47099/scenario' \
--header 'Content-Type: text/plain' \
--data "${BODY}"