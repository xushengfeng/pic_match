# pic_match

基于 WASM 的图像偏移检测库，使用相位相关（Phase Correlation）算法，支持部分重叠场景。

## API

```typescript
import { findOffset } from "pic_match";

const result = await findOffset(imageA, imageB);
// result: { dx: number, dy: number, confidence: number }
```

### 参数

| 参数     | 类型        | 说明                                                               |
| -------- | ----------- | ------------------------------------------------------------------ |
| `imageA` | `ImageData` | 参考图像，包含 `data`（Uint8ClampedArray RGBA）、`width`、`height` |
| `imageB` | `ImageData` | 待匹配图像，格式同上                                               |

### 返回值

| 字段         | 类型     | 说明                                               |
| ------------ | -------- | -------------------------------------------------- |
| `dx`         | `number` | X 方向偏移量（正数表示 imageB 相对于 imageA 右移） |
| `dy`         | `number` | Y 方向偏移量（正数表示 imageB 相对于 imageA 下移） |
| `confidence` | `number` | 置信度，值越高匹配越可靠                           |

## 性能

| 分辨率    | 耗时   |
| --------- | ------ |
| 64×64     | ~70ms  |
| 512×512   | ~350ms |
| 1920×1080 | ~300ms |
| 3840×2160 | ~400ms |

测试环境：Linux x86_64, Node.js 20

## 算法说明

使用相位相关（Phase Correlation）算法：

1. 将两张图像转为灰度图并降采样
2. 应用 Hanning 窗减少频谱泄漏
3. 对两张图分别做 2D FFT
4. 计算互功率谱（Cross-Power Spectrum）
5. 反 FFT 得到相关面，峰值位置即为偏移量

该算法对部分重叠场景有良好支持，即使两张图只有部分内容相同也能正确检测偏移。

## 开发环境准备

### 1. 安装 Rust 工具链

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

添加 WASM 编译目标：

```bash
rustup target add wasm32-unknown-unknown
```

### 2. 安装 wasm-pack

```bash
cargo install wasm-pack
```

### 3. 安装 Node.js 环境

推荐使用 pnpm，否则把下面相关命令中`pnpm`换成`npm`

### 4. 安装项目依赖

```bash
pnpm install
```

### 5. 环境检查

确认以下命令可用：

```bash
rustc --version      # >= 1.70
cargo --version
wasm-pack --version  # >= 0.12
node --version       # >= 18
```

## 构建

```bash
pnpm build
```

该命令依次执行：

1. `build:wasm` - 使用 wasm-pack 编译 Rust 为 WASM
2. `embed:wasm` - 将 WASM 二进制嵌入 TypeScript（base64）
3. `build:ts` - 使用 Vite 构建 TypeScript 库

## 测试

```bash
pnpm test
```

包含准确性测试和性能基准测试。
