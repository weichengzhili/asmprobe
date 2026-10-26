# `asmprobe` 项目结构说明文档

> 文档路径：`structure.md`（可放置于项目根目录或 `docs/` 下）
> 适用范围：`src/`、`tests/`、`benches/` 下所有 `.rs` 文件
> 约定：每个文件按 **职责 / 公开接口 / 依赖 / 设计约束** 四段描述，依赖方向只允许向下。

---

## 0. 分层总览

```
main.rs
  └─> cli/            (clap, domain, backend::factory, observability, config)
        └─> backend/  (domain, config; remote→reqwest, local→which/tempfile/tokio::process)
              └─> domain/   (纯类型，零 I/O)
config/ · output/ · observability/  为横切层，被上层引用，彼此不互相依赖。
```

| 层 | 允许依赖 | 禁止依赖 |
| :--- | :--- | :--- |
| `main.rs` | `cli`, `observability`, `error` | 任何 I/O crate、任何业务逻辑 |
| `cli/` | `domain`, `backend::factory`, `config`, `output` | `reqwest`, `std::process`, `tempfile` |
| `backend/` | `domain`, `config` | `cli`, `output` |
| `domain/` | 仅 `std` 与轻量枚举 | `serde`, `tokio`, `reqwest`, `clap` |
| `config/` | `serde`, `toml`, `directories` | `backend`, `cli` |
| `output/` | `domain` | 网络/进程调用 |
| `observability/` | `tracing` | 业务类型 |

---

## 1. 根文件

### `src/main.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 进程入口。解析 CLI、初始化 tracing、调用 `asmprobe::run`、把错误映射为退出码。 |
| **公开接口** | 仅 `fn main() -> std::process::ExitCode`。 |
| **依赖** | `clap::Parser`、`asmprobe::cli::Cli`、`asmprobe::observability`、`asmprobe::error`。 |
| **设计约束** | 不超过 20 行；禁止 `unwrap`/`expect`（除 `Cli::parse` 内部由 clap 处理）；禁止出现 `reqwest`、`tokio::process`、`std::fs`。错误打印统一走 `error::report`。 |

### `src/lib.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 库入口。声明所有模块、re-export 公共 API、提供进程内测试入口 `run(cli) -> Result<(), AppError>`。 |
| **公开接口** | `pub mod cli; pub mod backend; pub mod domain; pub mod config; pub mod output; pub mod observability; pub mod error;`<br>`pub fn run(cli: Cli) -> Result<(), AppError>` |
| **依赖** | 各子模块、`tokio`（`#[tokio::main]` 在 `run` 内部或由 `main` 提供 runtime）。 |
| **设计约束** | `run` 是唯一的编排点：`config::load` → `backend::factory::create` → `cli::commands::dispatch`。集成测试直接调 `run`，避免每次 spawn 二进制。 |

### `src/error.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 顶层应用错误 `AppError`，聚合各层错误并提供退出码映射与用户可读渲染。 |
| **公开接口** | `pub enum AppError { Cli(CliError), Config(ConfigError), Backend(BackendError), Io(std::io::Error) }`<br>`impl AppError { pub fn exit_code(&self) -> u8 }`<br>`pub fn report(e: &AppError) -> String` |
| **依赖** | `thiserror`、各层错误类型。 |
| **设计约束** | 退出码约定：`0` 成功、`1` 通用失败、`2` CLI 用法错误、`3` 网络/后端不可用、`4` 用户代码编译失败。映射表写入本文档附录，测试断言退出码。 |

---

## 2. `src/cli/` — 命令行层

### `src/cli/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 定义 `Cli`（clap derive 根结构）、全局参数（`--base-url`、`--log-level`、`--config`、`--format`）、`Commands` 枚举。 |
| **公开接口** | `pub struct Cli { command: Option<Commands>, backend: Option<BackendChoiceArg>, log_level: String, format: OutputFormatArg, config: Option<PathBuf> }`<br>`pub enum Commands { Compile(CompileArgs), ListLangs, ListCompilers(ListCompilersArgs), Repl, Config(ConfigArgs) }` |
| **依赖** | `clap`、`cli::args`、`cli::commands`。 |
| **设计约束** | 只做解析与类型化，不执行任何业务；全局参数通过 `#[arg(global = true)]` 保证子命令可用；所有 env 变量统一 `ASMPROBE_*` 前缀。 |

### `src/cli/args.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 值对象与 clap `ValueEnum` 定义：`BackendChoice`、`LanguageArg`、`OutputFormat`、`CompileArgs`、`ListCompilersArgs`、`ConfigArgs`。 |
| **公开接口** | `pub enum BackendChoice { Remote, Local }`（`ValueEnum`）<br>`pub struct CompileArgs { file: PathBuf, compiler: String, args: Vec<String>, backend: Option<BackendChoice>, stdin: bool }`<br>`impl From<LanguageArg> for domain::Language` |
| **依赖** | `clap`、`domain`。 |
| **设计约束** | CLI 字符串到 domain 类型的转换只在此文件发生；`--args` 接受多次出现或单次带空格的字符串，统一规范化为 `Vec<String>`。 |

### `src/cli/commands/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 子命令分发器：`dispatch(cli, backend, cfg) -> Result<(), AppError>`。 |
| **公开接口** | `pub async fn dispatch(cmd: Commands, backend: &dyn CompilerBackend, cfg: &Config) -> Result<(), AppError>` |
| **依赖** | 四个 command 模块、`backend::traits`、`config`。 |
| **设计约束** | 仅 `match` 分发，无业务逻辑；新增子命令只改此文件一行 + 新模块。 |

### `src/cli/commands/compile.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 执行 `compile`：读取源文件或 stdin → 调用 `backend.compile` → 通过 `output::formatter` 渲染到 stdout，诊断到 stderr。 |
| **公开接口** | `pub async fn execute(args: CompileArgs, backend: &dyn CompilerBackend, cfg: &Config) -> Result<(), AppError>` |
| **依赖** | `domain::{Language, CompilerSpec, SourceCode}`、`backend::traits`、`output::formatter`、`tokio::fs`。 |
| **设计约束** | 汇编只写 stdout；诊断与日志只写 stderr；`#[tracing::instrument]` 记录 `compiler`、`source_len`、`elapsed_ms`；`--stdin` 与 `file` 互斥由 clap `conflicts_with` 保证。 |

### `src/cli/commands/list_langs.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 列出支持的语言及默认参数。 |
| **公开接口** | `pub async fn execute(backend: &dyn CompilerBackend, format: OutputFormat) -> Result<(), AppError>` |
| **依赖** | `domain::Language`、`output::formatter`。 |
| **设计约束** | `--format json` 时输出机器可读 JSON 到 stdout；文本模式输出对齐表格。 |

### `src/cli/commands/list_compilers.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 列出某语言可用编译器：远程走 API，本地走 PATH 探测。 |
| **公开接口** | `pub async fn execute(args: ListCompilersArgs, backend: &dyn CompilerBackend, format: OutputFormat) -> Result<(), AppError>` |
| **依赖** | `backend::traits::list_compilers`、`output::formatter`。 |
| **设计约束** | 输出 `CompilerInfo { id, name, version, is_default }`；本地后端标注 `source = "local"`，远程标注 `source = "godbolt"`。 |

### `src/cli/commands/config.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 配置子命令：`get <key>`、`set <key> <value>`、`path`（打印配置文件路径）、`edit`（打开编辑器）。 |
| **公开接口** | `pub async fn execute(args: ConfigArgs, cfg: &mut Config) -> Result<(), AppError>` |
| **依赖** | `config::{schema, paths}`、`directories`。 |
| **设计约束** | `set` 写回前做 schema 校验并原子写入（临时文件 + rename）；禁止写入未知 key。 |

### `src/cli/commands/repl.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 交互式命令循环：逐行读取 → 切分 → `Cli::try_parse_from` → `dispatch`。 |
| **公开接口** | `pub async fn execute(backend: &dyn CompilerBackend, cfg: &mut Config) -> Result<(), AppError>`<br>`fn split(line: &str) -> Vec<String>`（私有，带单元测试） |
| **依赖** | `clap`、`std::io`、`cli::commands::dispatch`。 |
| **设计约束** | 整个会话只加载一次配置、只创建一个后端；提示符与提示语全部走 `stderr`，`stdout` 仍只承载命令结果；单行解析/执行失败只报错不退出；内置 `help` / `exit`；不引入新的第三方依赖（自带极简 shell 切分，支持引号与 `\` 转义）。 |

---

## 3. `src/backend/` — 后端层

### `src/backend/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 模块聚合与 re-export：`pub use traits::{CompilerBackend, BackendError}; pub use factory::create;` |
| **公开接口** | re-export 列表。 |
| **依赖** | `traits`、`factory`、`local`、`remote`。 |
| **设计约束** | 上层只允许通过 `backend::{CompilerBackend, create}` 使用本层，禁止 `backend::remote::*` 直连。 |

### `src/backend/traits.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 定义唯一的编译契约 `CompilerBackend` 与 `BackendError`。 |
| **公开接口** | `#[async_trait] pub trait CompilerBackend: Send + Sync { async fn compile(&self, spec: &CompilerSpec, src: &SourceCode, args: &[String]) -> Result<CompileOutput, BackendError>; async fn list_compilers(&self, lang: Language) -> Result<Vec<CompilerInfo>, BackendError>; }`<br>`pub enum BackendError { CompilerNotFound, Compilation { exit_code, diagnostics }, Network, Timeout, Io, Api }` |
| **依赖** | `async_trait`、`domain`、`thiserror`。 |
| **设计约束** | trait 签名不暴露 `reqwest::Response` 或 `std::process::Output`；`BackendError::Compilation` 与 `BackendError::Network` 必须可区分，供退出码映射。 |

### `src/backend/factory.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 根据 `BackendChoice` 与 `Config` 构造 `Box<dyn CompilerBackend>`。 |
| **公开接口** | `pub fn create(choice: BackendChoice, cfg: &Config) -> Result<Box<dyn CompilerBackend>, ConfigError>` |
| **依赖** | `local::LocalBackend`、`remote::GodboltBackend`、`config`。 |
| **设计约束** | 后端选择的唯一入口；CLI 层不出现 `if remote { … } else { … }`。 |

### `src/backend/local/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | `LocalBackend` 实现 `CompilerBackend`：解析编译器 → 构造命令 → 捕获输出 → 读取汇编。 |
| **公开接口** | `pub struct LocalBackend { resolver: Arc<dyn CompilerResolver> }`<br>`impl LocalBackend { pub fn new(cfg: &LocalConfig) -> Self; pub fn with_resolver(r: Arc<dyn CompilerResolver>) -> Self }` |
| **依赖** | `local::{resolver, command, temp}`、`domain`、`tokio::process`。 |
| **设计约束** | 编译进程使用 `tokio::process::Command` 或 `spawn_blocking`，避免阻塞 runtime；汇编写入 `NamedTempFile`，作用域结束自动清理；`with_resolver` 为测试注入 fake 编译器。 |

### `src/backend/local/resolver.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 编译器探测与版本解析：`which` 查找可执行文件，运行 `--version` 解析版本号并缓存。 |
| **公开接口** | `pub trait CompilerResolver: Send + Sync { fn resolve(&self, name: &str) -> Result<PathBuf, BackendError>; fn version(&self, path: &Path) -> Option<String>; }`<br>`pub struct SystemResolver { cache: Mutex<HashMap<String, PathBuf>> }` |
| **依赖** | `which`、`std::process`、`regex`（解析版本）。 |
| **设计约束** | 探测结果进程内缓存，避免重复 `which`；`list_compilers(Local)` 依次探测 `gcc`、`clang`、`rustc` 并标注可用性，不因某个缺失而失败。 |

### `src/backend/local/command.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 语言感知的命令行构造：`build(spec, source_path, asm_path, args) -> tokio::process::Command`。 |
| **公开接口** | `pub fn build(lang: Language, compiler: &Path, source: &Path, asm_out: &Path, user_args: &[String]) -> tokio::process::Command` |
| **依赖** | `domain::Language`、`tokio::process`。 |
| **设计约束** | C/C++ 用 `-S -o <asm>`；Rust 用 `--emit=asm -o <asm>`；`--intel-syntax` 时 C/C++ 追加 `-masm=intel`、Rust 追加 `-C llvm-args=-x86-asm-syntax=intel`；用户参数追加在默认参数之后，保证可覆盖。 |

### `src/backend/local/temp_workspace.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 临时源文件与汇编输出文件的生命周期封装。 |
| **公开接口** | `pub struct TempWorkspace { pub source: NamedTempFile, pub asm: NamedTempFile }`<br>`impl TempWorkspace { pub fn new(lang: Language, source: &str) -> io::Result<Self> }` |
| **依赖** | `tempfile`、`domain::Language`。 |
| **设计约束** | 源文件按语言加正确扩展名（`.c`/`.cpp`/`.rs`），否则 `rustc` 拒绝；两个文件在 `TempWorkspace` drop 时删除；不向用户暴露路径。 |

### `src/backend/remote/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | `GodboltBackend` 实现 `CompilerBackend`：调用 `client`，把 `dto` 转换为 `domain`。 |
| **公开接口** | `pub struct GodboltBackend { client: GodboltClient }`<br>`impl GodboltBackend { pub fn new(cfg: &RemoteConfig) -> Result<Self, ConfigError> }` |
| **依赖** | `remote::{client, dto}`、`domain`。 |
| **设计约束** | `dto` 到 `domain` 的 `From` 转换集中在此或 `dto.rs`；HTTP 错误与编译错误严格区分：HTTP 非 2xx → `BackendError::Network/Api`，JSON `code != 0` → `BackendError::Compilation`。 |

### `src/backend/remote/client.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | Godbolt HTTP 封装：base URL、超时、UA、重试、`/api/compilers/{lang}` 与 `/api/compiler/{id}/compile` 两个端点。 |
| **公开接口** | `pub struct GodboltClient { http: reqwest::Client, base: Url }`<br>`pub async fn compile(&self, id: &str, req: CompileRequest) -> Result<CompileResponse, BackendError>`<br>`pub async fn compilers(&self, lang: &str) -> Result<Vec<CompilerDto>, BackendError>` |
| **依赖** | `reqwest`（`json`、`rustls-tls`）、`remote::dto`、`tokio::time`。 |
| **设计约束** | 超时 30s；仅对幂等的 GET 与连接错误做一次重试，编译 POST 不重试以避免重复计费；`Accept: application/json` 固定；base URL 可指向自部署实例。 |

### `src/backend/remote/dto.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | Godbolt API 的请求/响应数据结构，以及到 `domain` 类型的转换。 |
| **公开接口** | `pub struct CompileRequest { source, user_arguments, filters, ... }`<br>`pub struct CompileResponse { asm, code, stderr, stdout, ... }`<br>`impl From<CompileResponse> for domain::CompileOutput` |
| **依赖** | `serde`、`domain`。 |
| **设计约束** | 所有字段 `#[serde(default)]` 或 `Option`，容忍 API 向后兼容新增字段；Godbolt 字段命名用 `rename` 对齐；本文件是唯一感知 Godbolt JSON 结构的地方。 |

---

## 4. `src/domain/` — 领域层

### `src/domain/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 聚合与 re-export：`Language`、`CompilerSpec`、`CompilerInfo`、`SourceCode`、`CompileOutput`、`Diagnostic`。 |
| **公开接口** | re-export 列表。 |
| **依赖** | 仅 `std`。 |
| **设计约束** | 本目录任何文件不得引入 `serde`、`tokio`、`reqwest`、`clap`；由 CI 的 `cargo-deny` + 自定义脚本检查。 |

### `src/domain/language.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | `Language` 枚举与语言策略：默认参数、源文件扩展名、Godbolt 语言 ID、汇编语法开关映射。 |
| **公开接口** | `pub enum Language { C, Cpp, Rust }`<br>`impl Language { pub fn default_args(&self) -> &'static [&'static str]; pub fn extension(&self) -> &'static str; pub fn godbolt_id(&self) -> &'static str }` |
| **依赖** | 无。 |
| **设计约束** | 所有语言相关默认值集中于此，禁止散落到 command builder 或 CLI；新增语言只改本文件 + 可能的 `command.rs` 分支。 |

### `src/domain/compiler.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | `CompilerSpec`（id 或路径 + 语言 + 版本）与 `CompilerInfo`（列表项，含 `source: Remote|Local`）。 |
| **公开接口** | `pub struct CompilerSpec { pub id_or_path: String, pub lang: Language }`<br>`pub struct CompilerInfo { pub id: String, pub name: String, pub version: Option<String>, pub source: CompilerSource, pub is_default: bool }` |
| **依赖** | `domain::Language`。 |
| **设计约束** | `CompilerSpec` 不区分远程 ID 与本地路径，由后端解释；`CompilerInfo::source` 让 `list-compilers` 输出能标注来源。 |

### `src/domain/output.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | `SourceCode`、`CompileOutput`、`Diagnostic` 三个核心值对象。 |
| **公开接口** | `pub struct SourceCode { pub text: String, pub origin: SourceOrigin }`<br>`pub struct CompileOutput { pub asm: String, pub diagnostics: Vec<Diagnostic>, pub exit_code: i32 }`<br>`pub struct Diagnostic { pub severity: Severity, pub message: String, pub line: Option<u32> }` |
| **依赖** | 无。 |
| **设计约束** | `CompileOutput` 同时承载成功与失败的编译结果（失败时 `asm` 为空、`exit_code != 0`），由上层决定是否视为错误；这样契约测试可对两个后端做同构断言。 |

---

## 5. `src/config/` — 配置层

### `src/config/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 配置加载与三层合并：CLI 覆盖 > 环境变量 > 配置文件 > 内置默认。 |
| **公开接口** | `pub fn load(cli: &Cli) -> Result<Config, ConfigError>`<br>`pub struct Config { pub remote: RemoteConfig, pub local: LocalConfig, pub output: OutputConfig }` |
| **依赖** | `config::{schema, paths}`、`serde`、`toml`、`directories`。 |
| **设计约束** | 合并逻辑只在此文件；`Config` 对后端只读；配置文件缺失时使用默认值而非报错，格式错误时报错并给出路径。 |

### `src/config/schema.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 配置文件的反序列化结构与默认值：`RemoteConfig { base_url, timeout_secs, retries }`、`LocalConfig { compiler_paths: HashMap<String, PathBuf>, default_args: ... }`、`OutputConfig { format, color }`。 |
| **公开接口** | `#[derive(Deserialize, Serialize, Default)] pub struct FileConfig { ... }`<br>`impl From<FileConfig> for Config` |
| **依赖** | `serde`、`toml`。 |
| **设计约束** | 所有字段 `#[serde(default)]`，保证旧配置文件向前兼容；`config set` 写入前用本结构反序列化校验。 |

### `src/config/paths.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 解析配置文件路径、缓存目录、日志目录，遵循 XDG / 平台约定。 |
| **公开接口** | `pub fn config_file() -> Result<PathBuf, ConfigError>`<br>`pub fn cache_dir() -> Result<PathBuf, ConfigError>` |
| **依赖** | `directories`。 |
| **设计约束** | Linux `~/.config/asmprobe/config.toml`、macOS `~/Library/Application Support/asmprobe/`、Windows `%APPDATA%\asmprobe\`；`ASMPROBE_CONFIG` 环境变量可覆盖。 |

---

## 6. `src/output/` — 渲染层

### `src/output/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 聚合与 re-export `formatter`、`highlight`。 |
| **公开接口** | re-export 列表。 |
| **依赖** | `formatter`、`highlight`。 |
| **设计约束** | 渲染层不做任何 I/O 之外的事：不读文件、不发请求。 |

### `src/output/formatter.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 把 `CompileOutput`、`Vec<CompilerInfo>`、`Vec<Language>` 渲染为文本表格或 JSON。 |
| **公开接口** | `pub fn render_compile(out: &CompileOutput, format: OutputFormat, color: bool) -> String`<br>`pub fn render_compilers(list: &[CompilerInfo], format: OutputFormat) -> String`<br>`pub fn render_languages(format: OutputFormat) -> String` |
| **依赖** | `domain`、`serde_json`、`comfy-table` 或手写对齐。 |
| **设计约束** | 文本模式下列宽按终端宽度自适应（`terminal_size`）；JSON 模式输出稳定 schema（字段顺序固定），作为机器接口契约由快照测试锁定。 |

### `src/output/highlight.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 汇编语法高亮（可选特性 `highlight`），基于 `syntect` 或轻量手写规则。 |
| **公开接口** | `pub fn highlight_asm(asm: &str, syntax: Syntax) -> String`（`Syntax::Att | Syntax::Intel`） |
| **依赖** | `syntect`（可选）、`domain`。 |
| **设计约束** | 默认编译不启用，`--features highlight` 开启；非 TTY 或 `NO_COLOR` 环境变量存在时自动禁用；高亮只影响 stdout，不影响 JSON 输出。 |

---

## 7. `src/observability/` — 可观测性层

### `src/observability/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 聚合与 re-export `tracing::init`。 |
| **公开接口** | `pub use tracing_init::init;` |
| **依赖** | `tracing_init`。 |
| **设计约束** | 全局只允许初始化一次，重复调用返回 `Err` 而非 panic。 |

### `src/observability/tracing.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 初始化 `tracing-subscriber`：`EnvFilter`、stderr writer、span 字段约定。 |
| **公开接口** | `pub fn init(level: &str) -> Result<(), InitError>`<br>约定 span 字段：`compiler`、`lang`、`source_len`、`elapsed_ms`、`backend`。 |
| **依赖** | `tracing`、`tracing-subscriber`（`env-filter`、`json` 可选）。 |
| **设计约束** | 日志**永远**写 stderr；`--format json` 且设置 `ASMPROBE_LOG_JSON=1` 时切换为 JSON 日志；span 字段名写入 `docs/architecture.md` 并由测试断言。 |

---

## 8. `tests/` — 测试

### `tests/cli_integration.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | `assert_cmd` 黑盒测试二进制：参数错误、文件不存在、stdout/stderr 归属、退出码、`--format json` schema。 |
| **公开接口** | 若干 `#[test]`。 |
| **依赖** | `assert_cmd`、`predicates`、`insta`、`tests/support`。 |
| **设计约束** | 完全离线：远程场景用 `support::mock_godbolt`，本地场景用 fake 编译器脚本注入 `LocalBackend::with_resolver`；禁止在默认测试中访问 `godbolt.org`。 |

### `tests/backend_contract.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 契约测试：同一组用例分别跑两个后端，断言返回的 `CompileOutput` 结构一致、错误分类一致。 |
| **公开接口** | `#[test] fn contract_compile_ok()`、`#[test] fn contract_compile_error()`、`#[test] fn contract_list_compilers()`。 |
| **依赖** | `asmprobe::backend::{LocalBackend, GodboltBackend, CompilerBackend}`、`tests/support`。 |
| **设计约束** | 用例数据放 `tests/fixtures/`；断言只针对 `domain` 类型，不针对具体汇编文本（不同编译器版本输出不同）。 |

### `tests/config_roundtrip.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 配置序列化/反序列化往返、三层优先级合并、未知 key 拒绝。 |
| **公开接口** | 若干 `#[test]`。 |
| **依赖** | `asmprobe::config`、`tempfile`、`insta`。 |
| **设计约束** | 用 `ASMPROBE_CONFIG` 指向临时文件，避免污染开发者本机配置。 |

### `tests/support/mod.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 测试辅助模块聚合：`mock_godbolt`、fake 编译器脚本生成、fixture 路径解析。 |
| **公开接口** | `pub mod mock_godbolt; pub fn fixture(name: &str) -> PathBuf; pub fn fake_compiler(dir: &Path, asm: &str) -> PathBuf` |
| **依赖** | `tempfile`、`std::fs`。 |
| **设计约束** | 仅测试编译；`#[cfg(test)]` 之外不可被 `src/` 引用。 |

### `tests/support/mock_godbolt.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | 用 `wiremock` 启动本地假 Godbolt，注册 `/api/compilers/{lang}` 与 `/api/compiler/{id}/compile` 的固定响应。 |
| **公开接口** | `pub async fn start() -> MockServer`<br>`pub async fn with_compile_error() -> MockServer` |
| **依赖** | `wiremock`、`tokio`。 |
| **设计约束** | 返回的 `MockServer::uri()` 作为 `RemoteConfig::base_url` 注入；fixture JSON 从 `tests/fixtures/` 读取，避免内联大段 JSON。 |

---

## 9. `benches/` — 基准

### `benches/parse_and_format.rs`

| 项 | 内容 |
| :--- | :--- |
| **职责** | Criterion 基准：大文件（10k 行汇编）的格式化耗时、JSON 序列化耗时、`EnvFilter` 解析耗时。 |
| **公开接口** | `fn bench_format(c: &mut Criterion)`、`criterion_group!`、`criterion_main!`。 |
| **依赖** | `criterion`、`asmprobe::output::formatter`。 |
| **设计约束** | 只在 `--features bench` 或 `cargo bench` 编译；基准数据从 `tests/fixtures/samples/large.s` 读取，避免基准文件膨胀仓库。 |

---

## 10. 文件职责速查表

| 文件 | 一句话职责 | 关键导出 |
| :--- | :--- | :--- |
| `main.rs` | 进程入口，三件事 | `main()` |
| `lib.rs` | 模块声明与编排入口 | `run()` |
| `error.rs` | 顶层错误与退出码 | `AppError`, `exit_code()` |
| `cli/mod.rs` | CLI 根结构与子命令枚举 | `Cli`, `Commands` |
| `cli/args.rs` | 值对象与 `ValueEnum` | `CompileArgs`, `BackendChoice` |
| `cli/commands/mod.rs` | 子命令分发 | `dispatch()` |
| `cli/commands/compile.rs` | 编译子命令编排 | `execute()` |
| `cli/commands/list_langs.rs` | 列语言 | `execute()` |
| `cli/commands/list_compilers.rs` | 列编译器 | `execute()` |
| `cli/commands/config.rs` | 配置读写 | `execute()` |
| `cli/commands/repl.rs` | 交互式命令循环 | `execute()`, `split()` |
| `backend/mod.rs` | 后端层聚合 | re-exports |
| `backend/traits.rs` | 编译契约 | `CompilerBackend`, `BackendError` |
| `backend/factory.rs` | 后端构造 | `create()` |
| `backend/local/mod.rs` | 本地后端实现 | `LocalBackend` |
| `backend/local/resolver.rs` | 编译器探测与版本 | `CompilerResolver`, `SystemResolver` |
| `backend/local/command.rs` | 命令行构造 | `build()` |
| `backend/local/temp_workspace.rs` | 临时文件生命周期 | `TempWorkspace` |
| `backend/remote/mod.rs` | 远程后端实现 | `GodboltBackend` |
| `backend/remote/client.rs` | HTTP 封装 | `GodboltClient` |
| `backend/remote/dto.rs` | API DTO 与转换 | `CompileRequest/Response` |
| `domain/mod.rs` | 领域层聚合 | re-exports |
| `domain/language.rs` | 语言策略 | `Language` |
| `domain/compiler.rs` | 编译器描述 | `CompilerSpec`, `CompilerInfo` |
| `domain/output.rs` | 编译结果值对象 | `CompileOutput`, `Diagnostic` |
| `config/mod.rs` | 配置加载与合并 | `load()`, `Config` |
| `config/schema.rs` | 配置文件结构 | `FileConfig` |
| `config/paths.rs` | 路径解析 | `config_file()`, `cache_dir()` |
| `output/mod.rs` | 渲染层聚合 | re-exports |
| `output/formatter.rs` | 文本/JSON 渲染 | `render_*()` |
| `output/highlight.rs` | 汇编高亮（可选） | `highlight_asm()` |
| `observability/mod.rs` | 可观测性聚合 | re-exports |
| `observability/tracing.rs` | tracing 初始化 | `init()` |
| `tests/cli_integration.rs` | 黑盒 CLI 测试 | `#[test]` |
| `tests/backend_contract.rs` | 后端契约测试 | `#[test]` |
| `tests/config_roundtrip.rs` | 配置测试 | `#[test]` |
| `tests/support/mod.rs` | 测试辅助 | `fixture()`, `fake_compiler()` |
| `tests/support/mock_godbolt.rs` | 假 Godbolt 服务 | `start()` |
| `benches/parse_and_format.rs` | 格式化基准 | `criterion_main!` |

---

## 附录 A：退出码约定

| 退出码 | 含义 | 来源 |
| :--- | :--- | :--- |
| `0` | 成功 | — |
| `1` | 通用失败（I/O、配置格式错误） | `AppError::Io`, `AppError::Config` |
| `2` | CLI 用法错误 | clap 自身 |
| `3` | 后端不可用（网络、超时、编译器未找到） | `BackendError::{Network, Timeout, CompilerNotFound, Api}` |
| `4` | 用户代码编译失败 | `BackendError::Compilation` |

该表由 `tests/cli_integration.rs` 逐项断言，确保脚本集成的稳定性。

补充约定：**不带任何子命令运行（`asmprobe`）视为“打印帮助”而非用法错误** —— 帮助内容走 `stdout`，退出码 `0`（`Cli::command: Option<Commands>` + `run` 内提前返回）。只有出现无法识别的子命令或非法参数时才由 clap 返回 `2`。

---

## 附录 B：stdout / stderr 契约

| 流 | 内容 | 禁止内容 |
| :--- | :--- | :--- |
| `stdout` | 汇编文本；`--format json` 时的机器可读 JSON（语言列表、编译器列表、编译结果）；`--help` 与无子命令时的帮助文本 | 日志、诊断、进度条 |
| `stderr` | `tracing` 日志、编译器诊断、用户错误提示、`indicatif` 进度、REPL 提示符/提示语 | 汇编、JSON 结果 |

该契约由 `tests/cli_integration.rs` 中 `stdout_is_asm_only` 与 `stderr_contains_diagnostics` 两个测试断言。