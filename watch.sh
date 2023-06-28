#!/usr/bin/env sh

trap "kill 0" SIGINT

just watch-frontend & P1=$!
P1=$!

just watch-backend & P2=$!
P2=$!

wait $P1 $P2
