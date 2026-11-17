# G19：SnarkMux 最终验证器 EVM 部署可行性

结论：**INCONCLUSIVE**；G19-A 状态：**BLOCKED_SOURCE**。

本次对作者原型地址进行了一次真实 HTTPS 访问，HTTP 状态为 401，curl 退出码 0。
有效地址：https://anonymous.4open.science/api/repo/SnarkMux/file/。原始响应头、响应体（base64）和 curl 日志均已保存；不是重用历史 HTTP 结果。

没有取得原型源码，无法锁定 commit、许可证、Spartan 变体、承诺方案或真实最终证明。按方案 4.1 停止后续部署测试。
HTTP 阻塞只描述本环境本次访问，不能写成“SnarkMux 不支持 EVM”，也不构成 NO_GO_WITHIN_BUDGET。

## 已完成与未完成

- 基于 G18 提交 `72ccc561d8934b0d761e17764ef9012bfe012626` 建立独立 G19 分支。
- 新保存来源清单、六项关系清单、通用 EVM 运算支持说明及本次访问证据。
- 检查 2931 个原有 Git 跟踪文件和外部论文正文 SHA-256，均未改动。
- 未构建原型、未生成证明、未写 Solidity、未发送 EVM 交易；所有密码学正反例均未运行，Gas/大小/时间指标为 null。
- 不使用历史 BLS12-381 证明充当本轮 BN254 测试，不复跑 G10/G17/G18，不开始 MicroSpartan 适配。

## 完整关系清单（要求，不是源码审计结论）

| 必需关系 | 检查式 | 本轮状态 |
|---|---|---|
| R1 Instance binding | `h = h_n` | 未取得源码，未审查/未执行 |
| R2 Recursive-state binding | `u_C,n.x = Hash(vk,n,h_n,u*,u*_C,n)` | 未取得源码，未审查/未执行 |
| R3 Final R1CS folding | `u′ = FoldR1CS.V(vk_FC,u_C,n,u*_C,n)` | 未取得源码，未审查/未执行 |
| R4 Target SNARK | `V_tgt(vk_FS,u*,pi*) = 1` | 未取得源码，未审查/未执行 |
| R5 Final Spartan proof | `SNARK′.V(vk_SNARK′,u′,pi′) = 1` | 未取得源码，未审查/未执行 |
| R6 Non-relaxed instance | `u_C,n is a valid non-relaxed instance` | 未取得源码，未审查/未执行 |

## EVM 支持边界

[EIP-196](https://eips.ethereum.org/EIPS/eip-196) 提供 BN254 G1 加法与标量乘法；[EIP-197](https://eips.ethereum.org/EIPS/eip-197) 提供配对乘积布尔检查，不暴露原始 GT 运算结果。
这只能用于通用依赖矩阵，不能证明原始 SnarkMux 的全部关系已经可部署。原型实际是否需要 G2/GT/Grumpkin/MSM/IPA 等及其规模均未知。
不能仅凭 Spartan 名称推定其 PCS。未给出未经源码支持的替代实现、复杂度或 Gas 估算。

## 固定研究门槛与来源

criteria.json 在本次源站请求前写入：原型最小 2 份证明、优先单交易、完整验证 ≤3,000,000 Gas、同条件 BN254 基线期望节省 ≥20%。本轮未评估是否满足。
环境见 environment.json；Prague/optimizer/viaIR/20M 交易和 30M 区块限额来自既有配置，不表示本轮运行了 VM。
原型 URL 的论文依据取自先前提交 `6db31eb2f7b6db706589ac1f6e8f27e2e106e1b9` 的归档（§6.2，印刷页26），本次没有重新审阅 PDF。该归档记录的是镜像 PDF，未确认与官方 PDF 逐字节相同。

## 复现与核对命令

`node --check scripts/g19_snarkmux_source_admission.mjs`

`node scripts/g19_snarkmux_source_admission.mjs`：本次真实执行入口；输出存在时拒绝覆盖。

`node scripts/g19_snarkmux_source_admission.mjs --check`：只读检查归档 SHA-256、旧文件及状态一致性；不会重新请求或运行证明。

## 继续所需证据

需要作者关联的可访问源码归档或仓库、可固定版本、许可证及匹配的最小最终证明向量。取得后另建新运行记录，先审查六项关系，再决定 G19-B；不要覆盖本次失败记录。
本轮到此停止；未自动提交或推送。
