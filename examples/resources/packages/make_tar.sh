#!/bin/bash

for dir in */; do
	dir_name=${dir%/}
	tar -cvf "${dir_name}.tar" "$dir_name"
done
