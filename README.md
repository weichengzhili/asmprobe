# asmprobe

工业级终端工具：把 **C / C++ / Rust** 源码编译产物（汇编、编译器诊断）从**本地工具链**或 **godbolt.org（Compiler Explorer）** 拉取并可视化，支持纯文本与 JSON 两种输出。

> 设计文档见 [`doc/structure.md`](doc/structure.md)。架构按 `domain / error / config / output / observability / backend / cli` 分层。

---

## 一、能做什么 / 提供的能力

| 能力 | 说明 |
| --- | --- |
| **本地编译看汇编** | 调用本机 `gcc` / `clang` / `rustc`，把源码编译成汇编，输出到 `stdout`；编译器警告/错误输出到 `stderr`（职责分离，方便 `> asm.s` 只截汇编）。 |
| **远程（godbolt）编译** | 通过 godbolt REST API（`/api/compilers`、`/api/compiler/<id>/compile`）编译，无需本地装编译器，便于对比不同编译器/版本。 |
| **多语言** | `c` / `cpp` / `rust`，按扩展名自动推断；也可用 `--lang` 显式指定（如 `--stdin` 时必填）。 |
| **stdin 管道** | `--stdin` 从标准输入读源码，配合 `--lang` 使用，适合与其他命令管道串联。 |
| **Intel 语法** | `--intel` 让 C/C++ 用 `-masm=intel`、Rust 用 `-C llvm-args=-x86-asm-syntax=intel` 输出 Intel 语法汇编。 |
| **交互式模式** | 不带子命令直接运行（终端里）或 `asmprobe repl` 进入交互循环：`compile a.c --intel`、`list-langs`、`config get ...` 直接敲，`help` 看用法，`exit` 退出。 |
| **列出语言 / 编译器** | `list-langs` 列出支持的语言；`list-compilers <lang>` 列出本地（PATH 中探测到的）或远程（godbolt）可用编译器。 |
| **三层配置合并** | 优先级：**CLI 参数/环境变量 > 配置文件 > 内置默认**；支持 `config get/set/path/edit`。 |
| **双输出格式** | `--format text`（默认，人读）/ `--format json`（机器读，含 `source`/`asm`/`diagnostics`/`exit_code`）/ `--format html`（源码-汇编对照页）。 |
| **结构化日志** | 走 `stderr`；`ASMPROBE_LOG_JSON=1` 切换为 JSON 日志，便于采集。 |
| **可观测性 & 退出码** | 编译失败、配置错误、用法错误有清晰区分的退出码（见下文）。 |

---

## 二、构建（Windows 原生 / WSL / Linux 均可）

> 本仓库已配置为 **Windows 原生零安装构建**：`rust-toolchain.toml` 固定到已安装的 `stable-x86_64-pc-windows-gnu` 工具链，`.cargo/config.toml` 把 gnu 目标的链接器指向 Rust 工具链**自带**的 `rust-lld`。因此直接 `cargo build` 即可生成 `.exe`，无需安装 `link.exe` / MinGW / 任何外部链接器。

### 方案 A：在 WSL 内直接编译（生成 Linux 二进制，在 WSL 内使用）

```powershell
# 1) 确认 WSL 已安装（默认发行版即可）
wsl -l -v

# 2) 进入 WSL，安装 Rust 工具链
wsl
# 在 WSL 内执行：
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 3) 进入项目（Windows 盘符自动挂载在 /mnt 下）
cd /mnt/c/你的路径/asmprobe
cargo build --release
# 产物：target/release/asmprobe   （Linux 二进制）
```

> WSL/Linux 默认已有 `gcc`。如需 godbolt 远程后端（`--features remote`），会拉取 `reqwest`/`ring`，需要网络与 C 编译器（`gcc`）；仅本地后端则无需 C 工具链。

### 方案 B：从 WSL 交叉编译出 Windows `.exe`（在 Windows 直接双击运行）

```powershell
wsl
source "$HOME/.cargo/env"

# 添加 Windows GNU 目标
rustup target add x86_64-pc-windows-gnu

# 安装 MinGW 交叉工具链（按发行版二选一）
#  Ubuntu/Debian:  sudo apt install -y mingw-w64
#  Alpine:        sudo apk add mingw-w64-gcc

# 交叉编译
cargo build --release --target x86_64-pc-windows-gnu
# 产物：target/x86_64-pc-windows-gnu/release/asmprobe.exe
```

### 默认构建（仅本地后端，零 C 工具链）

```powershell
cargo build --release                      # 产物 target/release/asmprobe.exe（本地后端，无需 C 编译器）
cargo build --release --features remote    # 额外加入 godbolt 远程后端（需要 gcc 编译 ring）
```

---

## 三、配置

配置文件位置（按 `directories` 标准目录，优先环境变量 `ASMPROBE_CONFIG` 指定路径）：

- Windows: `%APPDATA%\asmprobe\config.toml`
- Linux:   `$XDG_CONFIG_HOME/asmprobe/config.toml` 或 `~/.config/asmprobe/config.toml`

三层优先级：**CLI 参数 / 环境变量 > 配置文件 > 内置默认**。

| 环境变量 | 作用 |
| --- | --- |
| `ASMPROBE_CONFIG` | 指定配置文件路径 |
| `ASMPROBE_BASE_URL` | 覆盖 Godbolt 基址（默认 `https://godbolt.org`） |
| `ASMPROBE_FORMAT` | `text` / `json` / `html` |
| `ASMPROBE_LOG_LEVEL` | 日志级别（`error`/`warn`/`info`/`debug`/`trace`） |
| `ASMPROBE_LOG_JSON` | 设为 `1` 时日志输出 JSON |
| `ASMPROBE_COLOR` | 设为 `0` 时关闭彩色输出 |

---

## 四、命令参考

全局参数（任意子命令前均可加）：

```
asmprobe [--backend <local|remote>] [--format <text|json>] [--log-level <LEVEL>] [--config <PATH>] <子命令>
```

### 1. `compile` —— 编译并输出汇编

```
asmprobe compile [FILE] [--stdin] [--compiler <id>] [-a <ARGS>...] [--lang <c|cpp|rust>] [--intel]
```

- `[FILE]`：源文件路径（位置参数）。与 `--stdin` 互斥。
- `--stdin`：从标准输入读源码（需 `--lang`）。
- `--compiler <id>`：编译器标识，默认 `gcc`；godbolt 时填其编译器 id（如 `g142`、`clang1700`）。
- `-a, --args <ARGS>...`：透传给底层编译器的参数，支持以 `-` 开头的参数（如 `-a "-O2" "-mavx2"`）。
- `--lang <c|cpp|rust>`：显式指定语言（否则按扩展名推断）。
- `--intel`：输出 Intel 语法汇编。

行为约定：
- **汇编 → `stdout`**，**诊断（警告/错误）→ `stderr`**。
- 编译成功退出 `0`；编译器返回非零退出码时，工具退出 `1` 并把诊断打到 `stderr`。

示例：

```bash
# 本地 C → 汇编（默认 gcc）
asmprobe compile hello.c

# Rust（自动推断 .rs，用 rustc）
asmprobe compile main.rs

# 管道喂入 stdin，必须给 --lang
cat foo.c | asmprobe compile --stdin --lang c

# Intel 语法
asmprobe compile --intel demo.cpp

# 指定 clang，并透传优化参数
asmprobe compile --compiler clang demo.c -a "-O2" "-mavx2"

# 远程 godbolt（不需要本地编译器）
asmprobe --backend remote compile --compiler g142 demo.cpp

# 仅截取汇编（诊断仍在 stderr）
asmprobe compile demo.c > demo.s

# 机器可读 JSON
asmprobe --format json compile demo.c
```

### 2. `list-langs` —— 列出支持的语言

```bash
asmprobe list-langs
# 输出：c / cpp / rust
```

### 3. `list-compilers <LANG>` —— 列出可用编译器

```bash
# 本地：探测 PATH 中的 gcc/clang/rustc 等
asmprobe list-compilers rust

# 远程：列出 godbolt 上该语言的编译器
asmprobe --backend remote list-compilers c
```

### 4. `config` —— 配置读写

```
asmprobe config get <KEY>
asmprobe config set <KEY> <VALUE>
asmprobe config path
asmprobe config edit
```

常用键：

| 键 | 示例 |
| --- | --- |
| `remote.base_url` | `asmprobe config set remote.base_url https://godbolt.org` |
| `remote.timeout_secs` | `asmprobe config set remote.timeout_secs 30` |
| `remote.retries` | `asmprobe config set remote.retries 3` |
| `local.default_args` | `asmprobe config set local.default_args "-O2"` |
| `output.format` | `asmprobe config set output.format json` |
| `output.color` | `asmprobe config set output.color true` |

```bash
asmprobe config path                       # 显示配置文件路径
asmprobe config get remote.base_url        # 读取某个键
asmprobe config edit                       # 用 $EDITOR 打开配置文件
```

---

## 五、输出格式

### 文本（默认）

- `list-langs` / `list-compilers`：表格形式。
- `compile`：诊断（若有）打到 `stderr`，汇编文本打到 `stdout`。

### JSON（`--format json`）

`compile` 的 JSON 结构：

```json
{
  "source": "int add(int a, int b) {\n    return a + b;\n}\n",
  "asm": [
    { "text": "        push    rbp", "source": { "line": 3 } },
    { "text": "        mov     rbp, rsp", "source": { "line": 3 } },
    { "text": "        mov     eax, edi", "source": null }
  ],
  "diagnostics": [
    { "severity": "warning", "message": "unused variable 'x'", "line": 5 }
  ],
  "exit_code": 0
}
```

### HTML 对照页（`--format html`）

把**源码（左）与对应汇编（右）**渲染成单文件 HTML 对照页，按源码行对齐；页面内联 CSS、无任何外部依赖，双击即可在浏览器打开。

- 左栏为源码（带行号），右栏为汇编；每条汇编前的灰色数字即其对应的源码行号（来自编译器行号标记）。
- 与任何源码行都不关联的指令（如函数序言、`.cfi` 等）归入顶部一个无行号区块。
- 本地后端通过 `-fverbose-asm`（C/C++）与 `-g`（Rust）取得行号关联；godbolt 后端直接使用其返回的行号信息。

```bash
# 生成对照页并保存
asmprobe compile demo.c --format html > demo.html

# 浏览器打开（Windows）
start demo.html
```

---

## 六、日志与退出码

- 日志始终写到 **`stderr`**，不会污染 `stdout` 的汇编输出。
- 退出码：

| 退出码 | 含义 |
| --- | --- |
| `0` | 成功 |
| `1` | 编译失败（编译器非零退出）或配置错误 |
| `2` | 用法错误（参数缺失/冲突，如既给文件又 `--stdin` 且未给 `--lang`） |

---

## 七、典型工作流

```bash
# 1) 看某段 C 在 gcc 下生成的汇编，并只看汇编
asmprobe compile demo.c > demo.s

# 2) 对比同一段代码在 godbolt 上 clang 的汇编
asmprobe --backend remote compile --compiler clang1700 demo.c

# 3) 想用 Intel 语法且要机器可读
asmprobe --format json compile --intel demo.cpp > out.json

# 4) 在 CI 里用 JSON 解析诊断
asmprobe --format json compile demo.c | jq '.diagnostics'

# 5) 生成源码/汇编对照 HTML 页面（左源码、右汇编）
asmprobe compile demo.rs --format html > demo.html
```

---

## 八、开发 / 测试

```bash
cargo build                 # 默认仅本地后端（无需 C 工具链）
cargo build --features remote   # 含 godbolt 远程后端（需 gcc 编译 ring）
cargo test                  # 全部测试（含 godbolt 远程；离线时远程用例跳过）
cargo run -- list-langs --format json
cargo run -- compile example.c
cargo run -- compile --stdin --lang rust   # 从 stdin 读取
cargo run -- compile example.c --format html > example.html   # 生成源码/汇编对照页
```

> 测试覆盖：配置三层合并与持久化往返、CLI 离线黑盒（参数校验/退出码）、后端契约（本机有 `rustc` 时跑通一次真实编译，否则自动跳过）。
