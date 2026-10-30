# G1-E measured statistics

30 serial operation windows per B; 5 warmups excluded. 10,000-draw percentile-bootstrap median CIs are conditional on this fixed pool and machine, not independent-election uncertainty. P95 is deliberately unavailable at n=30.

| B | Individual median ms | Aggregate verify median ms | Aggregation median ms | Paired speed ratio (95% CI) |
|---|---:|---:|---:|---:|
| 2 | 0.967169 | 3.128145 | 4.360998 | 0.302 [0.300, 0.307] |
| 4 | 1.901342 | 4.091989 | 8.782294 | 0.481 [0.440, 0.504] |
| 8 | 3.944255 | 4.830629 | 16.769415 | 0.816 [0.768, 0.834] |
| 16 | 7.134917 | 5.522694 | 30.857372 | 1.402 [1.376, 1.432] |
| 32 | 15.446558 | 6.480822 | 52.507146 | 2.461 [2.373, 2.543] |

Memory: each B/operation has 3 fresh processes. Stage peaks are 1 ms sampled lower bounds including baseline, not exact allocator peaks; phase deltas are relative to pre-operation RSS. Cumulative process peaks are separately preserved. Timings exclude RSS sampling.

No new ballot proofs or Groth16 Setup; no current-suite state/P3/settlement/EVM experiment. Aggregate proofs remain larger; proof+matrix excludes cached VK/SRS and full framing. Gas/DA/calldata remain null.
