#!/usr/bin/bash

echo "rs"
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 5 --router-only --probing-rate 100000 > rs_0_8_16_5_100k.out 2> rs_0_8_16_5_100k.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 32 --gap 5 --router-only --probing-rate 100000 > rs_0_8_32_5_100k.out 2> rs_0_8_32_5_100k.err

echo "cxx"
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 16 --gaplimit 5 --probing_rate 100000 0.0.0.0/0 > cxx_0_8_16_5_100k.out 2> cxx_0_8_16_5_100k.err
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 32 --gaplimit 5 --probing_rate 100000 0.0.0.0/0 > cxx_0_8_32_5_100k.out 2> cxx_0_8_32_5_100k.err
