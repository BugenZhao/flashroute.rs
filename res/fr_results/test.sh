#!/usr/bin/bash

echo "split ttl rs"
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 8 --gap 5 --router-only > rs_0_8_8_5.out 2> rs_0_8_8_5.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 5 --router-only > rs_0_8_16_5.out 2> rs_0_8_16_5.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 32 --gap 5 --router-only > rs_0_8_32_5.out 2> rs_0_8_32_5.err

echo "split ttl cxx"
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 8 --gaplimit 5 0.0.0.0/0 > cxx_0_8_8_5.out 2> cxx_0_8_8_5.err
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 16 --gaplimit 5 0.0.0.0/0 > cxx_0_8_16_5.out 2> cxx_0_8_16_5.err
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 32 --gaplimit 5 0.0.0.0/0 > cxx_0_8_32_5.out 2> cxx_0_8_32_5.err


echo "gap rs"
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 0 --router-only > rs_0_8_16_0.out 2> rs_0_8_16_0.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 1 --router-only > rs_0_8_16_1.out 2> rs_0_8_16_1.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 2 --router-only > rs_0_8_16_2.out 2> rs_0_8_16_2.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 3 --router-only > rs_0_8_16_3.out 2> rs_0_8_16_3.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 4 --router-only > rs_0_8_16_4.out 2> rs_0_8_16_4.err

echo "gap cxx"
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 16 --gaplimit 0 0.0.0.0/0 > cxx_0_8_16_0.out 2> cxx_0_8_16_0.err
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 16 --gaplimit 1 0.0.0.0/0 > cxx_0_8_16_1.out 2> cxx_0_8_16_1.err
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 16 --gaplimit 2 0.0.0.0/0 > cxx_0_8_16_2.out 2> cxx_0_8_16_2.err
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 16 --gaplimit 3 0.0.0.0/0 > cxx_0_8_16_3.out 2> cxx_0_8_16_3.err
sudo ./flashroute --hitlist list_0_8.txt --split_ttl 16 --gaplimit 4 0.0.0.0/0 > cxx_0_8_16_4.out 2> cxx_0_8_16_4.err

echo "include hosts"
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 8 --gap 5 > rs_0_8_8_5h.out 2> rs_0_8_8_5h.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 5 > rs_0_8_16_5h.out 2> rs_0_8_16_5h.err
sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 32 --gap 5 > rs_0_8_32_5h.out 2> rs_0_8_32_5h.err

# sudo ./flashroute_rs -g 8 list_4_8.txt --no-dot --split-ttl 8 --gap 5 --router-only
# sudo ./flashroute_rs -g 8 list_0_8.txt --no-dot --split-ttl 16 --gap 5 --router-only > rs_0_8_16_5.out 2> rs_0_8_16_5.err