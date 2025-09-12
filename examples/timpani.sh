#!/bin/bash

BODY=$(< ./resources/timpani_test.yaml)

curl --location 'http://0.0.0.0:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY}"