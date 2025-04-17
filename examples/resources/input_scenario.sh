#!/bin/bash

echo "1: eco, 2:performance, 3:charging, 4: anti-enable, 5: anti-disable :"
read input
A=$((input))
BODY=""

if [ "$A" -eq 1 ]; then
        BODY="bms-eco"
elif [ "$A" -eq 2 ]; then
        BODY="bms-performance"
elif [ "$A" -eq 3 ]; then
        BODY="bms-charging"
elif [ "$A" -eq 4 ]; then
        BODY="antipinch-enable"
elif [ "$A" -eq 4 ]; then
        BODY="antipinch-disable"
fi

curl --location 'http://0.0.0.0:47099/scenario' \
--header 'Content-Type: text/plain' \
--data "${BODY}"