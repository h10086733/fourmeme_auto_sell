# FourMeme 自动卖币机器人

这是一个基于 Rust 和 ethers-rs 的自动卖币机器人，支持 FourMeme 平台的代币交易功能。

## 功能特性

### 1. 卖出现有代币 (默认模式)
- 自动检查代币余额
- 智能处理 approve 授权
- 预估卖出收益
- 分步执行，确保交易安全

### 2. 创建代币并购买 (新功能)
- 创建新的 meme 代币
- 自动计算初始价格
- 同步进行 approve 授权（如需要）
- 立即购买创建的代币

## 使用方法

### 环境设置

1. 安装 Rust（如果还没安装）：
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. 克隆项目并进入目录：
```bash
cd fourmeme_auto_sell
```

3. 编译项目：
```bash
cargo build --release
```

### 运行模式

#### 模式1: 卖出现有代币（默认）
```bash
cargo run
# 或
OPERATION_MODE=SELL cargo run
```

#### 模式2: 创建代币并购买
```bash
OPERATION_MODE=CREATE_AND_BUY cargo run
```

## 配置说明

### 参数配置

所有配置参数都在 `src/main.rs` 文件开头的配置部分。你可以根据需要修改这些参数：

```rust
// 网络配置
const CHAIN_ID: u64 = 56; // BSC链ID
const RPC_URL: &str = "your_rpc_url_here";

// 钱包配置
const PRIVATE_KEY: &str = "your_private_key_here";

// 合约地址配置
const TOKEN_MANAGER2_ADDRESS: &str = "0x5c952063c7fc8610FFDB798152D69F0B9550762b";
const TOKEN_MANAGER_HELPER_ADDRESS: &str = "0xF251F83e40a78868FcfA3FA4599Dad6494E46034";
const DEFAULT_TOKEN_ADDRESS: &str = "0x2b863b6fc88ce451cd5b31a2c5c049975fb84444";

// 创建代币配置
const CREATE_TOKEN_NAME: &str = "TestMeme";
const CREATE_TOKEN_SYMBOL: &str = "TMEME";
const CREATE_TOKEN_DESC: &str = "This is a test meme token created for demonstration purposes.";
const CREATE_TOKEN_LABEL: &str = "Meme"; // AI/Meme/Defi/Games/Infra/De-Sci/Social/Depin/Charity/Others

// 交易配置
const BUY_AMOUNT_BNB: f64 = 1.0; // 购买代币的BNB数量
const SLIPPAGE_PERCENT: u64 = 5; // 滑点保护百分比
```

### 合约地址（BSC 主网）
- **TokenManager2 (V2)**: `0x5c952063c7fc8610FFDB798152D69F0B9550762b`
- **TokenManagerHelper3 (V3)**: `0xF251F83e40a78868FcfA3FA4599Dad6494E46034`

### 旧版配置说明（已废弃）
~~在 `main.rs` 中的 `CREATE_AND_BUY` 模式下，可以修改以下参数~~（现在参数已移至文件开头的配置部分）

## 代码结构

### 主要方法

1. **`execute_sell_process`**: 完整的卖出代币流程
   - 检查代币余额
   - 获取代币信息
   - 处理 approve 授权
   - 执行卖出交易

2. **`sell_tokens_only`**: 纯粹的卖币方法（不含 approve 逻辑）
   - 直接执行卖出交易
   - 适用于已授权的情况

3. **`create_token_and_buy`**: 创建代币并购买
   - 创建新的 meme 代币
   - 预估购买成本
   - 处理 ERC20 授权（如需要）
   - 购买新创建的代币

### 结果结构体

- **`SellResult`**: 卖出操作结果
- **`CreateAndBuyResult`**: 创建并购买操作结果
- **`CreateTokenParams`**: 创建代币参数

## 安全特性

1. **分步执行**: approve 和 sell 分别执行，避免 MEV 攻击
2. **滑点保护**: 设置最小接收数量，防止价格滑点
3. **错误处理**: 完善的错误处理和日志输出
4. **余额检查**: 交易前后余额对比验证

## API 文档

项目基于 FourMeme 官方 API 文档实现，支持：

- TokenManager V1 (历史代币)
- TokenManager2 V2 (新代币)
- TokenManagerHelper3 V3 (统一接口)

详细 API 说明请参考 `api.doc` 文件。

## 注意事项

1. **私钥安全**: 请妥善保管私钥，不要在生产环境中硬编码
2. **测试环境**: 建议先在测试网络上测试
3. **Gas 费用**: 注意设置合适的 gas 价格
4. **网络状况**: 确保网络连接稳定

## 错误代码

常见错误代码参考：

- `GW - GWEI`: 精度不对齐到 GWEI
- `ZA - Zero Address`: 地址不能为零地址
- `TO - Invalid to`: 无效的接收地址
- `Slippage`: 滑点超出预期
- `More BNB`: BNB 余额不足

## 许可证

MIT License
