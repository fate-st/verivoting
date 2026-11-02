# G8-E：已有 G8-D 完整本地流程的成本归档

## 结论

G8-E PASS，属于已有真实记录的派生统计，不是新EVM实验。无需补运行。
G8-D已先保存为本地提交 `4a2cc163cd8b0d805d4a582c6e0d91586fc2627e`；旧结果仍保留实际执行的 git_dirty=true、旧Git SHA及源码快照。
本次不改合约、电路、配置或旧raw，不重新Setup/Prove、签名或生成CP包。

唯一计费来源：[G8-D winner_once.json](../experiments/g8d_raw/winner_once.json)。SHA-256：`20b72b4f23a66dcd8ef06e30c62494ad8f989d51107fb3c64876a9b1c1ff3268`。
该来源的第二次完整验收含21笔：17成功、4正常拒绝；首次归档失败仍留在G8-D，未当成第二个性能样本。

## 同版本、同一次执行的分类结果

| 阶段 | 笔数 | Gas | transaction data B | 签名交易序列化 B |
| --- | ---: | ---: | ---: | ---: |
| 部署 | 4 | 34,082,689 | 80,315 | 80,643 |
| 初始化绑定 | 1 | 63,614 | 4 | 103 |
| DKG/注册证书认证 | 4 | 10,593,712 | 1,232 | 1,640 |
| 两批B4接收 | 2 | 6,803,172 | 13,896 | 14,100 |
| 冻结CT2 | 1 | 204,081 | 4 | 103 |
| 冻结目标绑定及三份承诺认证 | 4 | 11,652,780 | 1,456 | 1,861 |
| 赢家验证并写入 | 1 | 2,073,560 | 644 | 746 |
| 拒绝测试（不计成功路径） | 4 | 2,263,025 | 1,936 | 2,341 |

- 部署：34,082,689 Gas，4笔；含内部CREATE及子验证器，不重复加费用。
- 非部署成功业务：31,390,919 Gas，13笔，包含初始化/认证/承诺，不仅是投票和披露。
- 成功路径含部署：65,473,608 Gas，17笔。
- 失败测试：2,263,025 Gas，4笔，单列不加进成功成本。

这里是含负例的同一次验收中成功收据的分类汇总，不是另跑的一条无负例路径；也不是一笔交易或一个区块。
Gas采用实际receipt总Gas，已考虑calldata floor，不能只把intrinsic与execution相加，或再叠加配对轨迹Gas。
环境沿用来源：Apple M4、v23.11.0、solc 0.8.30+commit.73712a01.Emscripten.clang、Prague、optimizer200/viaIR。

## 通信量口径：嵌套项不能重复相加

成功部署创建输入共80,315 B；成功业务calldata共17,236 B。
成功路径transaction data共97,551 B，签名交易序列化共99,196 B。
后者已经包含前者，不能相加；实际重复发送的payload按每笔计数，不作网络字节去重。

业务calldata的非重叠拆分：证明payload 5,632 B + 单票公开输入矩阵 6,400 B + 其他参数/selector/ABI 5,204 B = 17,236 B。
内部重建的2×37项状态输入和60项P3输入不是额外传入数组，不再各加一遍字段字节。

从成功交易ABI提取并按二进制SHA-256去重：11份证明共5,632 B；3份VK共18,688 B。
这些是已包含在交易输入内的公开二进制材料子集，不是另一笔额外通信，也不是完整离线协议通信。
Trustee私密通信、P2P封装/重传未测量，JSON以null/NOT_MEASURED记录，不填0；不把JSON文件大小叫最小协议通信。

## 离线密码学成本与状态绑定

复用8份单票、2份状态、1份P3证明；本轮Setup/Prove/新EVM交易/CP/签名执行数均为0。
原证明生成和DKG/私密CP成本并非0，而是REUSED_NOT_RETIMED，时间字段保留null。
沿用同一Receiver/Gate/Store、认证KeyCore、原G4-D VK、CT2 freeze seal及CP target；60项输入与G6/G7匹配。
原P3成功后Winner=1，写入及事件均一次；4个失败的业务状态原子不变。私有tally/opening/usable shares不输出。

## 必要检查与复现

脚本核对21笔签名/数据长度/收据，一一分类；逐笔、分组、总量守恒；成功与失败分离；ABI解码证明/矩阵/VK；赢家事件和冻结状态绑定。
G8-D清单63项核对当前原字节，6个中央状态文件核对已提交G8-D版本，避免更新状态时篡改旧清单。

```sh
node scripts/g8e_accounting.mjs --write
node scripts/g8e_accounting.mjs --check
node scripts/g8d_audit.mjs
```

--write仅新建或接受字节完全相同的派生产物；--check只读重算并逐字节核对JSON/本报告，不重新运行EVM。
派生JSON：[lifecycle_costs.json](../results/g8_e/lifecycle_costs.json)，SHA-256：`4b32db983dc24ce56669745fd4f1f23cf5e7d925ac6b28de0d46858953a239ae`。

## 停止边界

到G8-E停止。以上是固定D5/W32、两批B4的本地逐票Groth16备用路线成本，不是完整SnarkPack链上结算、完整系统离线+链上总成本或统计效率优势。
G9精确SnarkPack EVM尚未完成；G10公平基线/最终系统比较后置。本轮没有公链部署或新增图表/规模实验。
