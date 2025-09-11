#!/bin/bash

#BODY=$(< ./resources/helloworld.yaml)
BODY=$(< ./resources/timpani.yaml)

curl --location 'http://0.0.0.0:47099/api/artifact' \
--header 'Content-Type: text/plain' \
--data "${BODY}"