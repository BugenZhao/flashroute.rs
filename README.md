# flashroute.rs

A reproduction and enhancement of [paper](https://dl.acm.org/doi/10.1145/3419394.3423619) "*FlashRoute: Efficient Traceroute on a Massive Scale*" (ACM IMC'20) in Rust. [[Slides]](res/slides.pdf) [[Report]](res/report.pdf)

![fr](res/fr_report.png)

## FlashRoute
> FlashRoute is a tool to discover network topology, which is specially optimized for full Internet topology discovery. 
> It has high time efficiency in which it can finish the scan over the full IPv4 /24 address space in 7 minutes at probing speed of 200 Kpps, and 17 mins at probing speed of 100 Kpps.
> It also has high network efficiency, in which it can finish the scan using only 75% of probes used by Scamper and 30% of probes used by Yarrp to finish the same task.

More introductions for FlashRoute can be found in both the [paper](https://dl.acm.org/doi/10.1145/3419394.3423619) and the [repository](https://github.com/lambdahuang/FlashRoute).

## Reproduction in Rust
The original FlashRoute is written in C++14 with Boost and Abseil libraries, which works well but may still be not modern, concise, or safe enough.
We reimplement FlashRoute in Rust, a modern system programming language, and name it as *flashroute.rs*.

Compared to the original implementation, the main features of *flashroute.rs* are:
- 50% LoC compared to the orginal C++ implementation.
- Safer and extensible low-level network communication through *pnet*, instead of bare socket API.
- Asynchronous tasks and coroutine scheduling, instead of explicit thread management.
- More utilization of multi-core processors brings probing performance improvements of up to **40%** (tested on AMD EPYC 7B12 (8) @ 2.25 GHz).
- More comprehensive thread-safety thanks to the borrow checker of Rust.
- Mutex or rwlock free. All inter-task communications are achieved through message channels or atomic operations.
- `grain` option and hashmap-based data structure allows richer probing patterns.
- Produce human-readable results and even [visualization](res/fr_huge.png) of network topology.

## Usage
*flashroute.rs* requires Rust stable toolchain (>= 1.48.0). 
- Probe random hosts selected from each /30 (provided by `grain`) subnet of `202.120.0.0`, NIC specification is optional
    ```shell
    cargo run --release -- 202.120.0.0/16 --grain 2 [--interface en0]
    ```
- Probe random hosts selected from each /24 subnet of `0.0.0.0/0` (the internet)
    ```shell
    cargo run --release -- 0.0.0.0/0 --grain 8
    ```
- Probe all hosts of `115.159.2.0/24`
    ```shell
    cargo run --release -- 115.159.2.0/24 --grain 0
    ```
- Probe a single host
    ```shell
    cargo run --release -- 192.168.1.1/32 --grain 0
    ```
- Probe all hosts listed in a file line by line
    ```shell
    cargo run --release -- path/to/file --grain 8
    ```
- Follow the behavior of the original FlashRoute and only count the internet routers:
    ```shell
    cargo run --release -- 0.0.0.0/0 --grain 8 --router-only
    ```
Most of the options of original implementation are provided too, run `cargo run -- --help` to see all possible options.

### Notes

Listening on ICMP socket requires superuser permission, the *flashroute.rs* may automatically restart in sudo mode.

Windows users may be required to install pcap library to make it built. However, *flashroute.rs* has not been tested on Windows yet.

## References
1. Yuchen Huang, Michael Rabinovich, and Rami Al-Dalky. 2020. FlashRoute: Efficient Traceroute on a Massive Scale. In ACM Internet Measurement Conference (IMC ’20), October 27–29, 2020, Virtual Event, USA. ACM, New York, NY, USA, 13 pages. https://doi.org/10.1145/3419394.3423619
2. lambdahuang/Flashroute, https://github.com/lambdahuang/FlashRoute
