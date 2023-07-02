#!/usr/bin/env bash

cat $1 | sed 's/^rs//g' | sort -g | sed "s/^/rs/g" > tmp.tsv
mv tmp.tsv $1
