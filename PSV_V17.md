# PSV 第十七版：性能优化与配置式汉化支持

构建标识：`rfvp-psv-diag-20261022-17`。

## 分支与交付范围

`psv-v17-20261022` 保存已交付用户实机使用的第十七版源码快照，基于上游提交 `709b6ba2f9ae4ba9f34c56df82a044eb958ff329`。它包含第六版之后累计的性能、存读档输入恢复与第十七版汉化支持改动。

现有 `psv-diagnostics-main` 分支及其 PR 保留。该分支后续的 `config.toml` 改动没有合并进本次已验版本快照；本版读取下文的 `game.toml`。

- [第十七版 VPK](platform/vita/releases/v17/rfvp-psv-diagnostic-17.vpk)
- [安装包 SHA-256](platform/vita/releases/v17/SHA256SUMS.txt)
- [构建、源码及实机反馈记录](platform/vita/releases/v17/verification.json)

本仓库提供引擎和配置示例。游戏脚本、图片/音频包、汉化补丁及字体由使用者自行准备。

## 累计性能改动

- PSV 资源读取采用已知长度的 128KiB 分块直读。
- 通过 libdeflate 1.26 解压完整 zlib 资源，保留校验及输出长度检查。
- NEON 实现 BGR/BGRA 到 RGBA 的逐像素转换，包含标量尾部处理。
- 原生 C release 编译使用 `-O2`。
- generation 和脏区域驱动局部纹理更新，减少表情变化后的整图处理。
- 调整透明立绘的原生纹理策略，减少不必要的 CPU 缩放。
- 优化存档准备和资源恢复，成功读档后清理旧输入并隔离原按键的松开事件。
- 保留诊断计时、文字度量及音频诊断等此前改动。
- 第十六版已移除额外的 24MiB 已卸载表情包缓存；第十七版没有重新引入。

libdeflate 原始源码和 MIT 许可证位于 `crates/rfvp/vendor/libdeflate/`。固定版本 1.26，提交 `92e6a0db9fa848d742f9eb286c92afc60f2c3dda`。

## 第十七版新增功能

### 1. 明确选择脚本和编码

在 `ux0:data/rfvp/game/` 中放置 UTF-8 编码的 `game.toml`：

```toml
[game]
hcb = "MemoriaGBK.hcb"
nls = "gbk"
```

支持编码：`sjis`、`gbk`、`utf8`。没有配置时保持此前默认启动方式，包括旧的 `MemoriaGBK.gbk` 兼容入口。

配置解析器支持一个 `[game]` 表、带引号的单行字符串、注释与 UTF-8 BOM；不支持数组、字符串转义或其他表。路径使用相对路径及 `/` 分隔符。旧式平台的 `rfvp.toml` 仍独立保留。

### 2. 原脚本与翻译表使用独立编码

对于采用外部偏移翻译表的汉化，可以使用：

```toml
[game]
hcb = "Sakura.hcb"
nls = "sjis"
text_patch = "patch.dat"
text_nls = "gbk"
hcb_crc32 = "9974eff7"
asset_aliases = "chs-assets.tsv"
```

原脚本和日文资源名称仍按 ShiftJIS 处理，仅翻译内容按 GBK 解码。读取 PushString 时按原字符串位置查找翻译，原 HCB 的字节码、跳转地址和资源包不需要改写。

支持的偏移翻译格式：小端 `u32` 记录数，每条为 `u32` 脚本字符串位置、`u16` 字节长度和文本内容；非空文本以 NUL 结尾，零长度表示保留原文。压缩格式在该数据前放置四个小端 `u32`：零标识、时间戳、解压大小、压缩大小，随后为 zlib 数据。

启用翻译表要求明确指定 HCB、原脚本 CRC32 和译文编码。启动时检查大小、原脚本版本、PushString 位置、重复偏移、文本结束符及编码。解压后的翻译内容与查询索引在运行期间保留，不预解压或缓存立绘。

### 3. 配置式图片重定向

`chs-assets.tsv` 为 UTF-8 文本，每行两个 VFS 路径，以一个制表符分隔：

```text
graph/title_logo1	patch/title_logo1
```

例如上面的目标由现有 `patch.bin` 包提供。别名目标需要存在，重复映射和映射链会报错。资源继续按需读取、解压和上传。

本次本地数据检查覆盖 57,053 条翻译记录，其中 53,917 条替换文本、3,136 条保留原文；本地 Windows 补丁对应 71 项图片映射。以上数量属于用户提供的补丁数据，不是引擎中的固定判断条件。

格式参考：[公开补丁生成工具](https://github.com/elementary-particle/my-sakura-moyu/blob/f06f3c16cdc5fc45bcf2da98d3f19cab432277c9/create_patch.py)。加载代码独立实现；Windows 警告脚本注入、Direct3D 效果及外挂字幕逻辑未移植。

## 使用与字体

游戏文件放在 `ux0:data/rfvp/game/`，中文字体命名为 `default.ttf`。第十七版可在 VitaShell 覆盖安装，应用 ID 为 `RFVPPSV01`，SFO 显示版本为 `00.00`，请以构建标识区分。

用户最新本地部署选择了“联想小新黑体 常规”。替换字体只需要覆盖 `default.ttf` 并重启，VPK 不变。字体未包含在仓库或安装包中。

切换游戏时使用各自完整的游戏目录、配置和存档；先备份当前文件。汉化图片和翻译表需与原脚本版本匹配。

## 构建

沿用仓库 VitaSDK 和 Rust 工具链设置，在仓库根目录运行：

```sh
bash platform/vita/build_vita.sh
```

正常构建产物位于 `target/rfvp-psv/psv/rfvp.vpk`。本分支附带的诊断安装包还包含源码差分及许可证信息；其 SHA-256 用于核对已交付安装包，不表示不同环境构建必然逐字节一致。

## 验证记录

- 第十五版：用户确认首次立绘明显改善、画面正常。
- 第十六版：用户确认卡顿已在可接受范围内，设备日志记录 libdeflate 和 NEON 路径生效。
- 第十七版：2026-10-22 用户反馈“第17版我已在实机确认，没有问题”。反馈未逐项列出游戏或功能，因此不扩展为所有游戏、路线与存档场景均已通过。
- 第十七版完成 PSV 交叉编译、VPK CRC/SELF/SFO、构建标识和源文件哈希检查；本轮同步没有新增或运行实现测试、游戏回归，也没有重新编译。

安装包 SHA-256：`f608706eb0cd0aa7bfd216fa4ee2980831b0866f01f7ad25134d5a3031fc38e2`。
