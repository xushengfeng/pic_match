# pic_match

基于 WASM 的图像偏移检测库，使用相位相关（Phase Correlation）算法，支持部分重叠、不同尺寸的匹配。

## 安装

```bash
npm install picture_match
```

## API

```typescript
import { findOffset } from "pic_match";

const result = await findOffset(imageA, imageB);
// result: { dx, dy, confidence, overlap, peak, psr }
```

### 参数

| 参数     | 类型        | 说明                                                               |
| -------- | ----------- | ------------------------------------------------------------------ |
| `imageA` | `ImageData` | 参考图像，包含 `data`（Uint8ClampedArray RGBA）、`width`、`height` |
| `imageB` | `ImageData` | 待匹配图像，格式同上                                               |

两张图尺寸可以不同；只要内容处于**同一像素尺度**，且重叠面积足够（约 15% 以上），就能求出偏移。

### 返回值

| 字段         | 类型     | 说明                                                                  |
| ------------ | -------- | --------------------------------------------------------------------- |
| `dx`         | `number` | X 方向偏移量（正数表示 imageB 相对于 imageA 右移），带亚像素精度       |
| `dy`         | `number` | Y 方向偏移量（正数表示 imageB 相对于 imageA 下移）                     |
| `confidence` | `number` | 匹配程度：重叠区归一化互相关（NCC），1 表示完全一致，<0.4 基本不可信   |
| `overlap`    | `number` | 重叠面积占较小一张图的比例，0~1                                       |
| `peak`       | `number` | 互功率谱峰值高度，调试用                                              |
| `psr`        | `number` | 峰值旁瓣比，调试用                                                    |

`confidence` 的含义是"重叠区域看起来有多像"，因此被遮挡、亮度/局部内容差异都会让它下降；
当它很低时说明结果不可信（尺度不同、内容无关、重叠太小等），应当直接丢弃这次匹配。

```typescript
const { dx, dy, confidence } = await findOffset(a, b);
if (confidence >= 0.8) {
  // 可信：把 B 放到 (dx, dy) 处即可与 A 对齐
}
```

## 性能

| 分辨率    | 耗时    |
| --------- | ------- |
| 64×64     | ~1ms    |
| 256×256   | ~14ms   |
| 512×512   | ~70ms   |
| 1920×1080 | ~74ms   |
| 3840×2160 | ~95ms   |
| 1920×1080 里找 120px 小图 | ~71ms |
| 3840×2160 里找 120px 小图 | ~82ms |

测试环境：Linux x86_64, Node.js 26, release 构建。耗时主要由内部 FFT 画布尺寸决定（上限 1024×1024），
超过该尺寸的图像会先降采样；小图会保留至少约 32px，避免被大图"拖"到无法匹配。

## 算法说明

使用相位相关（Phase Correlation）算法：

1. 将两张图像转为灰度图，按需降采样
2. 各自去均值，并施加两端渐变的 Tukey 窗（比 Hanning 窗保留更多有效面积，对部分重叠更友好）
3. 对两张图分别做 2D FFT
4. 计算互功率谱并归一化（PHAT），丢掉幅度只保留相位
5. 反 FFT 得到相关面，只在物理可达的偏移范围内取峰
6. 对若干候选峰用重叠区 NCC 复核，取一致性最高的一个（避免低重叠时的伪峰）
7. 对选中的峰做抛物线插值，得到亚像素偏移

该算法对部分重叠场景有良好支持，即使两张图只有部分内容相同也能正确检测偏移。

### 适用与不适用

| 场景                                   | 结果 |
| -------------------------------------- | ---- |
| 同尺度平移（含部分重叠，重叠 ≥15%）    | ✅    |
| 两张图尺寸/长宽比不同，但像素尺度相同  | ✅    |
| 大图里找小图（模板匹配）               | ✅    |
| 亮度、对比度变化                       | ✅    |
| 局部遮挡                               | ✅ 位置正确，`confidence` 下降 |
| 旋转、透视                             | ❌    |
| 同一内容的不同分辨率/缩放（需先统一尺度） | ❌ `confidence` 会很低 |

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
2. `build:ts` - 使用 Vite 构建 TypeScript 库

## 测试

```bash
pnpm build:wasm   # 测试直接加载 pkg/ 里的 WASM，需要先构建
pnpm test
```

测试覆盖：

| 文件                | 内容                                                         |
| ------------------- | ------------------------------------------------------------ |
| `match.test.ts`     | 基础平移、负向偏移、不同画布尺寸、大偏移、无关/纯色图像        |
| `overlap.test.ts`   | 部分重叠扫描（重叠 15%~98%）、两图互不包含、全景拼接          |
| `template.test.ts`  | 大图找小图：居中/边缘/右半屏、亮度变化、遮挡、1080p 图标搜索   |
| `accuracy.test.ts`  | 打印各场景的误差、`confidence`、`overlap`，直观看到匹配程度    |
| `benchmark.test.ts` | 各分辨率的耗时，并检查不超过预算                              |

Rust 侧另有单元测试：

```bash
cargo test
```
