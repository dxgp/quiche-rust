#!/bin/bash

for (( ; ; )); 
do
    NUM=`shuf -i 10000-200000 -n 1`
    echo $NUM
    sudo wondershaper s1-eth1 $NUM $NUM
    sleep 0.01
done