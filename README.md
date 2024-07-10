# QUIC Performance Evaluation

This project aims to re-evaluate the claims made in the paper "Taking a Long Look at QUIC". I assessed QUIC's performance under various network conditions, comparing it with TCP and analyzing its behavior when parameters such as link size, loss rates, and jitter are varied.

## Project Overview

- **Implementation**: Used QUICHE, a QUIC implementation by Cloudflare, instead of Chromium's QUIC implementation used in the original paper.
- **Topology**: Created a dumbbell-shaped topology using Mininet, consisting of 4 hosts connected to two switches, with a bottleneck link between the switches.

## Methodology

I evaluated QUIC's performance by varying the following parameters:

- Bottleneck bandwidth: 10Mbps, 50Mbps, 100Mbps
- File Size: 5KB, 10KB, 100KB, 200KB, 500KB, 1MB, 10MB, 100MB
- Loss: 0%, 1%
- Congestion Control Algorithms:
  - QUIC: BBR, BBRv2, CUBIC, RENO
  - TCP: CUBIC
- RTT: 112ms
- Jitter: 0ms, 20ms

## Key Findings

1. **Fairness**: QUIC showed unfairness to TCP, with the fairness depending on which flow started first.
2. **Performance**: 
   - QUIC's performance benefits were mainly observed in high bandwidth, low loss, and large transfer size configurations.
   - In high loss environments, QUIC underperformed compared to TCP.
   - QUIC's performance was severely degraded in high jitter environments.
3. **Variable Bandwidth**: QUIC provided moderate performance gains in variable bandwidth environments.

## Conclusions

- QUIC without streams provides some performance gains over TCP, but significant improvements are expected with the introduction of streams and the elimination of head-of-line blocking.
- QUIC underperforms in high loss and high jitter environments when considering single stream performance.
- The performance gains of QUIC are more pronounced in variable bandwidth environments, especially when used in conjunction with HTTP/3 streams.

