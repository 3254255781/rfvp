# RFVP PSV 诊断与性能优化

本分支基于 [xmoezzz/rfvp](https://github.com/xmoezzz/rfvp) 的固定提交 [`709b6ba2f9ae4ba9f34c56df82a044eb958ff329`](https://github.com/xmoezzz/rfvp/tree/709b6ba2f9ae4ba9f34c56df82a044eb958ff329)，记录 2026-10-19 对《星空的记忆》PS Vita 移植版的实机诊断、性能优化和汉化脚本入口。

本分支只包含引擎源码改动和本说明，**不包含游戏脚本、图片/音频资源包、字体、存档或第三方汉化补丁文件**。

## 摘要

- 相对固定基线共 **19 个源码文件**：16 个修改、3 个新增，文本差分 **+579 / −129** 行。
- 新增有界诊断报告：34 个阶段计时、最慢帧历史、资源加载事件、音频/硬件错误计数。
- 定位并处理的主要问题：
  - 存档准备约 338ms → 102–104ms：缩略图改为直接采样帧缓冲的双线性缩放，并拆分截图/快照/整理计时。
  - 对白文字光栅化：每个字 4 次 `font.rasterize` → 1 次，实机样本 `text_raster` 约 31–73ms。
  - 配音临时静音未在下一次播放时清除：修正通道播放状态，保留分类音量。
  - PSV 汉化脚本入口：存在 `MemoriaGBK.gbk` 时按 GBK 解码并枚举 `.gbk`，否则保持原 `.hcb` + ShiftJIS。
- 本机独立验收：61 项测试通过、1068 组字体度量一致、日文 14/14 张历史画面逐字节一致、中文 GBK 脚本 4200 帧通过。
- 第六版中文流程已用 Droid Sans Fallback 在 PSV 实机确认运行正常；上游 `main` 已继续前进，本分支未合并最新主线。

## 基线与范围

- 基线：`xmoezzz/rfvp` 提交 `709b6ba2f9ae4ba9f34c56df82a044eb958ff329`，基线源码 ZIP 的 SHA-256 为 `9abf89756d688858d3eb975603b3004f81ed342c945be5fa3d1d160c329bd99d`。
- 对照时上游 `main` 为 `99435f22e94eaec27c0c1b9f22cc2a6b1910d7d1`，相对基线已前进 17 个提交；本分支没有把这些上游改动算作本地改动，也没有合并它们。
- 审查 1090 个基线文件、1093 个当前文件，排除 `target`、`.git`、`__pycache__` 等目录。
- 引擎外的构建脚本、日志、VPK、WSL 虚拟磁盘和协作文件不计入 19 个源码文件，也不在本分支中。

## 改动摘要

| 版本 | 主要改动 | 结果 / 范围 |
| --- | --- | --- |
| 01 | 接通现有 perf 计时；加入读包、解码、差分、纹理上传、音频诊断；保留慢帧历史 | 定位 CPU 缩图、CPU 图片副本恢复、零音量等问题 |
| 02 | 轻度缩小的纹理交给 GPU；RGBA 输入借用；失败时回退旧尺寸 | 慢帧中的 CPU 纹理缩图耗时消失；部分 GPU 内存占用增加 |
| 03 | 下一次播放时清除上一句临时静音，保留分类音量 | 角色配音恢复 |
| 04 | 存档缩略图直接采样帧缓冲；拆分截图、快照、存档整理计时 | `save` 约 338ms → 102–104ms |
| 05 | 纯度量改用 `Font::metrics`；单字绘制复用 `item_adv` | 文字光栅化明显下降；1068 组度量一致、日文 14/14 截图一致 |
| 06 | 存在 `MemoriaGBK.gbk` 时选择 GBK 和 `.gbk` 扩展名 | 本机中文 4200 帧通过；PSV 实机（Droid Sans Fallback）已确认 |

## 诊断系统

- `crates/rfvp/src/perf.rs`：阶段扩展到 34 项，增加作用域计时和带时间戳的事件。
- `platform/vita/host/src/diagnostics.rs`：有界慢帧历史（最慢 8 帧、最近 12 个超过 50ms 的帧）、阶段汇总、音频累计统计；报告文件写入在测量帧之外。
- `platform/vita/host/src/main.rs`：启用诊断时钟并按周期写出 `debug_report.txt`/`debug_boot.txt`；`diagnostics_off.txt` 可关闭记录。
- `platform/vita/host/src/audio.rs`、`c/rfvp_psv_vitasdk.c`：记录混音/加载/播放错误、主线程等待音频锁、硬件线程/端口/音量返回值和计数。
- `crates/rfvp/src/subsystem/resources/{vfs,graph_buff,parts_manager,motion_manager}.rs`：记录慢读、图片解码、CPU 副本恢复、差分合成和纹理上传路径/耗时。

## 关键改动

### 存档缩略图（04）

`capture_thumbnail` 不再复制整幅画面后做原面积平均滤波，而是直接读取帧缓冲，用四邻点双线性采样生成缩略图；保存路径新增 `psv_thumbnail`、`save_snapshot`、`save_finalize` 分段计时。缩略图尺寸和存档结构保持原约定，细节/抗锯齿效果可能不同。

### 文字度量（05）

`text_manager::measure_char_advance` 和 `char_draw_bounds` 原先调用 `font.rasterize` 后丢弃位图。现在改用上游已有的 `Font::metrics`（相同的 `glyph_metrics_internal`，不生成覆盖位图）；单字绘制仍走一次 `draw_char`，并复用已计算的 `item_adv`、扣回字间距。排版和像素路径保持不变；共享文字管理器也影响其他平台，本轮只覆盖普通 no_std 路径。

### 配音临时静音（03）

原逻辑把“下一句播放前清除临时静音”实现成持续保留，导致角色语音以 0 音量播放。修正后下一次播放清除临时静音，同时保留用户设置的分类音量。

### 汉化脚本入口（06）

原流程按目录顺序取第一份 `.hcb` 并按 ShiftJIS 解码，不保证选中汉化脚本，也不会按文件名切换编码。第六版在 PSV 入口检查 `ux0:data/rfvp/game/MemoriaGBK.gbk`：

- 存在：`boot.hcb_extension = "gbk"`、`boot.nls = Nls::GBK`；
- 不存在：保持默认 `.hcb` + ShiftJIS。

这是当前游戏的约定文件名开关，不是通用汉化自动识别；核心按扩展名枚举后取第一份，应只保留一份 `.gbk` 脚本。GBK 解码器、按扩展名枚举和字体回退均沿用上游实现。

### 纹理与内存策略（02）

`texture_policy.rs` 对符合尺寸限制的轻度缩小使用 GPU 过滤，避免 CPU 面积缩图；RGBA 输入尽量减少复制；失败或内存不足时回退原尺寸。原尺寸纹理可能占用更多 GPU 内存，缩略图细节也可能与旧面积滤波不同。

## 源码差分

下表为相对固定基线的全部源码差分；路径为仓库相对路径。

| 文件 | 状态 | 行数 | 内容 |
| --- | --- | ---: | --- |
| [crates/rfvp/src/audio_player/channels.rs](crates/rfvp/src/audio_player/channels.rs) | 修改 | +83 / −2 | 下一次播放清除临时静音；零音量来源日志和 3 项回归测试 |
| [crates/rfvp/src/no_std_core.rs](crates/rfvp/src/no_std_core.rs) | 修改 | +29 / −22 | 音频命令错误细化；存档快照/整理计时 |
| [crates/rfvp/src/perf.rs](crates/rfvp/src/perf.rs) | 修改 | +28 / −1 | 34 阶段、作用域计时、带时间戳事件 |
| [crates/rfvp/src/rfvp_audio/commands.rs](crates/rfvp/src/rfvp_audio/commands.rs) | 修改 | +1 / −0 | 音频源路径和流编号 |
| [crates/rfvp/src/subsystem/components/syscalls/parts.rs](crates/rfvp/src/subsystem/components/syscalls/parts.rs) | 修改 | +4 / −2 | 差分加载/选择错误写入诊断 |
| [crates/rfvp/src/subsystem/components/syscalls/sound.rs](crates/rfvp/src/subsystem/components/syscalls/sound.rs) | 修改 | +2 / −2 | 临时静音语义注释 |
| [crates/rfvp/src/subsystem/resources/graph_buff.rs](crates/rfvp/src/subsystem/resources/graph_buff.rs) | 修改 | +4 / −0 | 图片解码/CPU 副本恢复计时和路径 |
| [crates/rfvp/src/subsystem/resources/motion_manager/mod.rs](crates/rfvp/src/subsystem/resources/motion_manager/mod.rs) | 修改 | +2 / −0 | 差分合成计时和编号事件 |
| [crates/rfvp/src/subsystem/resources/parts_manager.rs](crates/rfvp/src/subsystem/resources/parts_manager.rs) | 修改 | +2 / −0 | 多帧差分解码计时和路径 |
| [crates/rfvp/src/subsystem/resources/text_manager.rs](crates/rfvp/src/subsystem/resources/text_manager.rs) | 修改 | +9 / −11 | 纯度量改用 metrics；单字复用 item_adv |
| [crates/rfvp/src/subsystem/resources/vfs.rs](crates/rfvp/src/subsystem/resources/vfs.rs) | 修改 | +20 / −4 | 慢读、读取失败、配音路径 |
| [platform/vita/host/c/rfvp_psv_vitasdk.c](platform/vita/host/c/rfvp_psv_vitasdk.c) | 修改 | +18 / −3 | 音频线程/端口/音量/硬件输出计数 |
| [platform/vita/host/src/audio.rs](platform/vita/host/src/audio.rs) | 修改 | +47 / −5 | 混音/加载/播放错误、锁等待、音频事件 |
| [platform/vita/host/src/diagnostics.rs](platform/vita/host/src/diagnostics.rs) | 新增 | +115 / −0 | 有界慢帧历史、阶段汇总、音频统计 |
| [platform/vita/host/src/lib.rs](platform/vita/host/src/lib.rs) | 修改 | +2 / −0 | 导出纹理尺寸策略和缩略图模块 |
| [platform/vita/host/src/main.rs](platform/vita/host/src/main.rs) | 修改 | +58 / −46 | 诊断时钟/周期报告；`.gbk` 汉化入口 |
| [platform/vita/host/src/render.rs](platform/vita/host/src/render.rs) | 修改 | +47 / −31 | 尺寸策略、RGBA 借用/回退、缩略图直接采样 |
| [platform/vita/host/src/texture_policy.rs](platform/vita/host/src/texture_policy.rs) | 新增 | +17 / −0 | 轻度缩小交给 GPU；保留大幅缩小/内存不足回退 |
| [platform/vita/host/src/thumbnail.rs](platform/vita/host/src/thumbnail.rs) | 新增 | +91 / −0 | 裁切双线性缩略图采样及 5 项单元测试 |

## 验证

本轮独立验收在本机完成，使用软件渲染器和测试音频接口，**不等同于 PSV GPU、硬件发声或实机帧率**：

- 10 项诊断/缩略图/纹理尺寸策略测试通过。
- 1 项字体层 `metrics`/`rasterize` 度量一致性测试通过，覆盖 1068 组“字体 × 字符 × 字号”。
- 49 项引擎 no_std 核心测试通过，包含日文脚本 4200 帧、存读档、音频状态。
- 中文 GBK 脚本分别配 SimHei 与 Droid Sans Fallback，各 4200 帧运行测试通过。
- 用户 PSV 实机确认：第六版 + `MemoriaGBK.gbk` + Droid Sans Fallback 中文文本完整、运行正常；SimHei/SimSun 在实机缺字（SimSun 为 TTC 集合），不作为推荐组合。
- 日文历史画面 14/14 张逐字节一致。
- 第五、六版 VPK 的 ZIP、SELF 文件头、版本标识、应用参数、SHA-256 和包内补丁独立核对通过；第六版补丁与当前源码差异一致。

安装包标识（构建产物不随本仓库发布，仅记录哈希）：

| 版本 | 字节数 | SHA-256 |
| --- | ---: | --- |
| 05 | 917311 | `2a15e8804e4f960b7b57a5b746a4294db7ceeb5923ff7f340fbaa3f50b993ac7` |
| 06 | 917744 | `e7f401eda5edda3a515d5e72a31f991c6c422ca02f870a638faeae6b2f1424b1` |

## 构建与使用

- 构建沿用上游 PSV 的 VitaSDK/cargo-vita 流程；本分支不包含作者的 WSL 构建脚本和测试夹具。
- 日文：保留 `.hcb` + ShiftJIS，默认路径不变。
- 中文：在 `ux0:data/rfvp/game/` 放置唯一一份 `MemoriaGBK.gbk`，并用覆盖简体中文字形的单文件 TTF 作为 `default.ttf`。推荐 **Droid Sans Fallback**（4,033,420 字节，单 TTF，Apache-2.0；本机 4200 帧验证，SHA-256 `acb6440a713d880a13a21b468ba7cd43f5a2b2934972e51be791c880730777b8`）。PSV 实机反馈 SimHei/SimSun 缺字；SimSun 是 `.ttc` 字体集合，不建议直接改名使用。复制后核对文件大小/哈希。安装第六版后检查 `debug_boot.txt`：
  - `build=rfvp-psv-diag-20261019-06`
  - `hcb=MemoriaGBK.gbk`
- 切换前备份存档。中文脚本通常使用独立存档目录，但核心存在旧式 `game/save/` 迁移逻辑，跨脚本兼容未验证；建议新开游戏。
- 本仓库不包含 VPK、游戏脚本、资源包、字体或存档，需用户用自己的合法游戏数据构建/安装。

## 已知限制

1. **中文字体验证范围**：第六版 + Droid Sans Fallback 已在 PSV 实机确认本次测试场景正常；其他中文字体、稀有字、全流程/全场景和中文存档兼容仍未穷尽。SimHei/SimSun 在实机缺字，SimSun 是 TTC 集合，不能直接改名当单 TTF 使用。
2. **热点仍存在**：存档约 102ms、整图上传约 59ms、CPU 图片副本恢复约 700ms、音频同步加载长帧仍有日志证据。
3. **旧存档跨脚本兼容未验证**：文件名和标题参与存档目录计算，另有旧式迁移逻辑。
4. **诊断有开销**：报告周期、资源事件和线程计数适合排查；帧时间排除了报告整理/写入，不能直接当作无诊断版本的端到端帧率。
5. **质量/内存权衡**：GPU 缩放和缩略图可能改变细节或增加 GPU 内存占用。
6. **未合并上游最新 main**：本分支针对固定旧基线，与当前 main 的冲突和回归未验收。
7. **共享代码影响面**：文字度量和音频通道改动位于共享代码；本轮只覆盖普通 no_std 路径，未测试所有平台和 `old_school` 配置。
8. **汉化入口是游戏专用约定**：只识别 `MemoriaGBK.gbk`，不是通用汉化自动识别；扩展名枚举取第一份，需只保留一份 `.gbk`。

## 许可与第三方说明

- 本分支基于 MPL-2.0 的 rfvp 源码，修改部分沿用同一许可证；请保留上游版权和许可证声明。
- 仓库不包含《星空的记忆》的游戏脚本、图片、音频、字体、存档，也不包含第三方汉化补丁文件；这些内容由用户自行准备。
- 游戏名称、商标和第三方汉化文本归各自权利人所有；本仓库仅包含引擎侧的加载/诊断/性能改动。
