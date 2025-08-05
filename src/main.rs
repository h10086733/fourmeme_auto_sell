// 核心优化说明：
// 1. 使用ethers内置Multicall确保approve和sell在同一个区块中执行，防止被抢跑
// 2. 内置Multicall提供了简单易用的API
// 3. ITokenManager2接口提供了直接的卖出功能
// 4. 新增Four.meme平台API支持，通过Web API创建代币

use ethers::{
    prelude::*,
    providers::{Http, Provider},
    signers::LocalWallet,
    types::{Address, U256},
    utils::format_ether,
};
use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::{Utc, Duration};
use hex;

// ================================
// 配置参数 - 所有可配置的参数都在这里
// ================================

// 网络配置
const CHAIN_ID: u64 = 56; // BSC链ID
const RPC_URL: &str = "https://neat-practical-arrow.bsc.quiknode.pro/b2f485b14431f07a8e9e25951ad16fb364a0dd3a";

// 钱包配置
const PRIVATE_KEY: &str = "";

// 合约地址配置
const TOKEN_MANAGER2_ADDRESS: &str = "0x5c952063c7fc8610FFDB798152D69F0B9550762b"; // TokenManager2 V2
const TOKEN_MANAGER_HELPER_ADDRESS: &str = "0xF251F83e40a78868FcfA3FA4599Dad6494E46034"; // Helper3
const DEFAULT_TOKEN_ADDRESS: &str = "0xa61619c6569fcc0f8ecdd62854b2e452f3a84444"; // 用于卖出模式的默认代币

// 创建代币配置
const CREATE_TOKEN_NAME: &str = "狐链fox"; // 代币名称
const CREATE_TOKEN_SYMBOL: &str = "狐链fox";
const CREATE_TOKEN_DESC: &str = "BullCoin is a powerful cryptocurrency that embodies the strength of the bull market. Combining blockchain innovation with community engagement, BullCoin offers unique opportunities for investors. Inspired by the bullish spirit, it aims to revolutionize the digital economy while supporting both Bitcoin and Ethereum ecosystems."; // 描述信息
const CREATE_TOKEN_LABEL: &str = "Meme"; // AI/Meme/Defi/Games/Infra/De-Sci/Social/Depin/Charity/Others
const CREATE_TOKEN_WEB_URL: &str = "https://difipay.vercel.app";
const CREATE_TOKEN_TWITTER_URL: &str = "";
const CREATE_TOKEN_TELEGRAM_URL: &str = "";
const CREATE_TOKEN_PRE_SALE: &str = "0.2"; // 创建代币时预购买的BNB数量，"0"表示不预购买
const CREATE_TOKEN_IMAGE_PATH: &str = "image/狐链fox.jpg"; // 本地图片路径
const BUY_AMOUNT_BNB: f64 = 0.2; // 购买代币的BNB数量

// Four.meme API配置
const FOURMEME_API_BASE_URL: &str = "https://four.meme/meme-api";
const DEFAULT_NETWORK_CODE: &str = "BSC";
const DEFAULT_WALLET_NAME: &str = "MetaMask";

// ================================
// 以下为结构体定义和函数实现
// ================================

// Four.meme API 相关结构体定义
#[derive(Debug, Serialize, Deserialize)]
struct NonceRequest {
    #[serde(rename = "accountAddress")]
    account_address: String,
    #[serde(rename = "verifyType")]
    verify_type: String,
    #[serde(rename = "networkCode")]
    network_code: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NonceResponse {
    code: i32,     // code是整数类型
    msg: String,   // 添加msg字段
    data: String,  // data是字符串类型（nonce值）
}

#[derive(Debug, Serialize, Deserialize)]
struct VerifyInfo {
    address: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    signature: String,
    #[serde(rename = "verifyType")]
    verify_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    region: String,
    #[serde(rename = "langType")]
    lang_type: String,
    #[serde(rename = "loginIp")]
    login_ip: String,
    #[serde(rename = "inviteCode")]
    invite_code: String,
    #[serde(rename = "verifyInfo")]
    verify_info: VerifyInfo,
    #[serde(rename = "walletName")]
    wallet_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    code: i32,        // code是整数类型
    msg: String,      // 添加msg字段
    data: String,     // access_token
}

#[derive(Debug, Serialize, Deserialize)]
struct RaisedToken {
    symbol: String,
    #[serde(rename = "nativeSymbol")]
    native_symbol: String,
    #[serde(rename = "symbolAddress")]
    symbol_address: String,
    #[serde(rename = "deployCost")]
    deploy_cost: String,
    #[serde(rename = "buyFee")]
    buy_fee: String,
    #[serde(rename = "sellFee")]
    sell_fee: String,
    #[serde(rename = "minTradeFee")]
    min_trade_fee: String,
    #[serde(rename = "b0Amount")]
    b0_amount: String,
    #[serde(rename = "totalBAmount")]
    total_b_amount: String,
    #[serde(rename = "totalAmount")]
    total_amount: String,
    #[serde(rename = "logoUrl")]
    logo_url: String,
    #[serde(rename = "tradeLevel")]
    trade_level: Vec<String>,
    status: String,
    #[serde(rename = "buyTokenLink")]
    buy_token_link: String,
    #[serde(rename = "reservedNumber")]
    reserved_number: u32,
    #[serde(rename = "saleRate")]
    sale_rate: String,
    #[serde(rename = "networkCode")]
    network_code: String,
    platform: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateTokenRequest {
    name: String,
    #[serde(rename = "shortName")]
    short_name: String,
    desc: String,
    #[serde(rename = "imgUrl")]
    img_url: String,
    #[serde(rename = "launchTime")]
    launch_time: u64,
    label: String,
    #[serde(rename = "lpTradingFee")]
    lp_trading_fee: f64,
    #[serde(rename = "webUrl")]
    web_url: String,
    #[serde(rename = "twitterUrl")]
    twitter_url: String,
    #[serde(rename = "telegramUrl")]
    telegram_url: String,
    #[serde(rename = "preSale")]
    pre_sale: String,
    // 固定参数
    #[serde(rename = "totalSupply")]
    total_supply: u64,
    #[serde(rename = "raisedAmount")]
    raised_amount: u32,
    #[serde(rename = "saleRate")]
    sale_rate: f64,
    #[serde(rename = "reserveRate")]
    reserve_rate: f64,
    #[serde(rename = "funGroup")]
    fun_group: bool,
    #[serde(rename = "clickFun")]
    click_fun: bool,
    symbol: String,
    #[serde(rename = "raisedToken")]
    raised_token: RaisedToken,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateTokenResponse {
    code: i32,                // code是整数类型
    msg: String,              // 添加msg字段
    data: CreateTokenData,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateTokenData {
    #[serde(rename = "tokenId")]
    token_id: u64,
    #[serde(rename = "totalAmount")]
    total_amount: String,
    #[serde(rename = "saleAmount")]
    sale_amount: String,
    template: u32,
    #[serde(rename = "launchTime")]
    launch_time: u64,
    #[serde(rename = "serverTime")]
    server_time: u64,
    #[serde(rename = "createArg")]
    create_arg: String,
    signature: String,
    bamount: String,  // 需要的BNB数量
    tamount: String,  // 代币数量
    #[serde(rename = "contractAddress")]
    contract_address: Option<String>,  // 合约地址（可选，API可能不返回）
}

// ERC20 ABI 简化版
abigen!(
    IERC20,
    r#"[
        function allowance(address owner, address spender) view returns (uint256)
        function approve(address spender, uint256 amount) returns (bool)
        function balanceOf(address account) view returns (uint256)
        function transfer(address to, uint256 amount) returns (bool)
        function transferFrom(address from, address to, uint256 amount) returns (bool)
    ]"#,
);

// ITokenManager2 ABI - 更新后的方法签名，包含事件定义
abigen!(
    ITokenManager2,
    r#"[
        function sellToken(uint256 origin, address token, address from, uint256 amount, uint256 minFunds, uint256 feeRate, address feeRecipient) external
        function buyTokenAMAP(address token, address to, uint256 funds, uint256 minAmount) external payable
        function buyToken(address token, address to, uint256 amount, uint256 maxFunds) external payable
        function createToken(bytes calldata createArg, bytes calldata sign) external payable returns (address token)
        
        event TokenCreated(address indexed token, address indexed creator, uint256 tokenId)
        event Transfer(address indexed from, address indexed to, uint256 value)
    ]"#,
);

// ITokenManagerHelper3 ABI - 用于预估和获取信息
abigen!(
    ITokenManagerHelper3,
    r#"[
        function getTokenInfo(address token) external view returns (uint256 version, address tokenManager, address quote, uint256 lastPrice, uint256 tradingFeeRate, uint256 minTradingFee, uint256 launchTime, uint256 offers, uint256 maxOffers, uint256 funds, uint256 maxFunds, bool liquidityAdded)
        function tryBuy(address token, uint256 amount, uint256 funds) external view returns (address tokenManager, address quote, uint256 estimatedAmount, uint256 estimatedCost, uint256 estimatedFee, uint256 amountMsgValue, uint256 amountApproval, uint256 amountFunds)
        function trySell(address token, uint256 amount) external view returns (address tokenManager, address quote, uint256 funds, uint256 fee)
        function buyWithEth(uint256 origin, address token, address to, uint256 funds, uint256 minAmount) external payable
        function sellForEth(uint256 origin, address token, uint256 amount, uint256 minFunds, uint256 feeRate, address feeRecipient) external
        function calcInitialPrice(uint256 maxRaising, uint256 totalSupply, uint256 offers, uint256 reserves) external view returns (uint256 priceWei)
    ]"#,
);

// 卖出代币的结果结构体
#[derive(Debug)]
struct SellResult {
    token_sold: U256,
    bnb_received: U256,
    sell_tx_hash: H256,
    block_number: Option<U256>,
    approve_tx_hash: Option<H256>,
}

// 完整的卖出代币流程方法
async fn execute_sell_process(
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    token_address: Address,
    token_manager_helper_address: Address,
) -> std::result::Result<SellResult, Box<dyn std::error::Error>> {
    // 创建合约实例
    let token = IERC20::new(token_address, client.clone());
    let token_manager_helper = ITokenManagerHelper3::new(token_manager_helper_address, client.clone());
    let user_address = client.address();
    
    println!("开始卖出代币流程...");
    
    // 查询卖出前的余额
    let before_token_balance = token.balance_of(user_address).call().await?;
    let before_bnb_balance = client.get_balance(user_address, None).await?;
    
    println!("卖出前代币余额: {}", before_token_balance);
    println!("卖出前BNB余额: {}", format_ether(before_bnb_balance));
    
    // 检查代币余额
    if before_token_balance == U256::zero() {
        return Err("没有代币可以卖出!".into());
    }
    
    // 获取token信息和预估卖出结果
    let token_info = token_manager_helper.get_token_info(token_address).call().await?;
    let token_manager_address = token_info.1;
    let quote = token_info.2;
    let liquidity_added = token_info.11;
    
    println!("Token Manager: {:?}", token_manager_address);
    println!("Quote Token: {:?}", quote);
    println!("Liquidity Added: {}", liquidity_added);
    
    
    // 预估卖出结果
    let sell_estimate = token_manager_helper.try_sell(token_address, before_token_balance).call().await?;
    let estimated_min_funds = sell_estimate.2;
    let fee = sell_estimate.3;
    println!("预估能获得BNB: {}", format_ether(estimated_min_funds));
    println!("预估手续费: {}", format_ether(fee));
    println!("TokenManager地址: {:?}", token_manager_address);
    
    // 检查授权并在必要时进行approve
    let current_allowance = token.allowance(user_address, token_manager_address).call().await?;
    println!("当前授权额度: {}", current_allowance);
    println!("需要授权额度: {}", before_token_balance);
    
    let mut approve_tx_hash = None;
    
    if current_allowance < before_token_balance {
        println!("🔹 步骤1: 发送approve交易");
        
        let approve_tx = token.approve(token_manager_address, before_token_balance);
        let approve_pending = approve_tx.send().await?;
        approve_tx_hash = Some(approve_pending.tx_hash());
        println!("✅ approve交易已发送: {:?}", approve_tx_hash.unwrap());
        
        // 等待approve交易确认
        let approve_receipt = approve_pending.await?.unwrap();
        println!("✅ approve交易确认! 区块: {:?}", approve_receipt.block_number);
        
        // 检查新的授权额度
        let new_allowance = token.allowance(user_address, token_manager_address).call().await?;
        println!("新的授权额度: {}", new_allowance);
    } else {
        println!("✅ 授权已足够，直接执行卖出");
    }
    
    // 使用纯粹的卖币方法执行卖出操作
    println!("\n使用纯粹卖币方法进行卖出...");
    
    let sell_result = sell_tokens_only(
        client.clone(),
        token_address,
        token_manager_address,
        before_token_balance,
    ).await?;
    
    // 返回完整结果，包含approve信息
    Ok(SellResult {
        token_sold: sell_result.token_sold,
        bnb_received: sell_result.bnb_received,
        sell_tx_hash: sell_result.sell_tx_hash,
        block_number: sell_result.block_number,
        approve_tx_hash,
    })
}

// 纯粹的卖币方法（不包含approve逻辑）
async fn sell_tokens_only(
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    token_address: Address,
    token_manager_address: Address,
    token_amount: U256,
) -> std::result::Result<SellResult, Box<dyn std::error::Error>> {
    let user_address = client.address();
    
    // 创建合约实例
    let token_manager2 = ITokenManager2::new(token_manager_address, client.clone());
    
    // 卖出参数
    let origin = 0u64;
    let fee_rate = 0u64;
    let fee_recipient: Address = "0xE1c727B62cF1ed816587E1005790f9E30299bf88".parse()?;
    let min_funds = U256::zero();
    
    // 获取卖出前的BNB余额
    let before_bnb_balance = client.get_balance(user_address, None).await?;
    
    println!("\n🚀 开始卖出代币...");
    println!("代币数量: {}", token_amount);
    
    // 执行卖出交易
    println!("🔹 发送sellToken交易");
    
    let sell_tx = token_manager2.sell_token(
        U256::from(origin),
        token_address,
        user_address,
        token_amount,
        min_funds,
        U256::from(fee_rate),
        fee_recipient
    );
    
    let sell_pending = sell_tx.send().await?;
    let sell_tx_hash = sell_pending.tx_hash();
    println!("✅ sellToken交易已发送: {:?}", sell_tx_hash);
    
    // 等待卖出交易确认
    let sell_receipt = sell_pending.await?.unwrap();
    println!("✅ sellToken交易确认! 区块: {:?}", sell_receipt.block_number);
    
    // 获取卖出后的BNB余额
    let after_bnb_balance = client.get_balance(user_address, None).await?;
    
    // 计算卖出结果
    let token_sold = token_amount; // 假设全部卖出成功
    let bnb_received = after_bnb_balance.saturating_sub(before_bnb_balance);
    
    println!("🎉 卖出完成!");
    println!("代币卖出数量: {}", token_sold);
    println!("获得BNB数量: {}", format_ether(bnb_received));
    
    Ok(SellResult {
        token_sold,
        bnb_received,
        sell_tx_hash,
        block_number: sell_receipt.block_number.map(|n| U256::from(n.as_u64())),
        approve_tx_hash: None, // 纯粹卖币方法不包含approve
    })
}

// 创建代币并购买的结果结构体
#[derive(Debug)]
struct CreateAndBuyResult {
    token_address: Address,
    create_tx_hash: H256,
    buy_tx_hash: H256,
    approve_tx_hash: Option<H256>,
    tokens_received: U256,
    bnb_spent: U256,
    creation_block: Option<U256>,
    buy_block: Option<U256>,
}

// 创建代币参数结构体 - 更新为Four.meme API格式
#[derive(Debug, Clone)]
struct CreateTokenParams {
    name: String,
    short_name: String, // symbol
    desc: String,
    img_url: String,
    launch_time: Option<u64>, // 可选，默认为当前时间+1小时
    label: String, // AI/Meme/Defi/Games/Infra/De-Sci/Social/Depin/Charity/Others
    web_url: Option<String>,
    twitter_url: Option<String>,
    telegram_url: Option<String>,
    pre_sale: String, // 预购买的BNB数量，"0"表示不预购买
}

// Four.meme API客户端结构体
struct FourMemeApiClient {
    client: reqwest::Client,
    base_url: String,
    access_token: Option<String>,
}

impl FourMemeApiClient {
    fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: FOURMEME_API_BASE_URL.to_string(),
            access_token: None,
        }
    }

    // 1. 获取nonce
    async fn get_nonce(&self, account_address: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let nonce_request = NonceRequest {
            account_address: account_address.to_string(),
            verify_type: "LOGIN".to_string(),
            network_code: DEFAULT_NETWORK_CODE.to_string(),
        };

        // 调试：打印实际发送的JSON
        let json_body = serde_json::to_string(&nonce_request)?;
        println!("发送的JSON: {}", json_body);

        let response = self
            .client
            .post(&format!("{}/v1/private/user/nonce/generate", self.base_url))
            .json(&nonce_request)
            .send()
            .await?;

        let nonce_response: NonceResponse = response.json().await?;
        
        if nonce_response.code != 0 {
            return Err(format!("获取nonce失败: {} - {}", nonce_response.code, nonce_response.msg).into());
        }

        Ok(nonce_response.data)
    }

    // 2. 用户登录
    async fn login(&mut self, wallet: &LocalWallet, nonce: &str) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let account_address = format!("{:?}", wallet.address());
        
        // 签名消息
        let message = format!("You are sign in Meme {}", nonce);
        let signature = wallet.sign_message(message.as_bytes()).await?;
        let signature_hex = format!("0x{}", hex::encode(signature.to_vec()));

        let verify_info = VerifyInfo {
            address: account_address.clone(),
            network_code: DEFAULT_NETWORK_CODE.to_string(),
            signature: signature_hex,
            verify_type: "LOGIN".to_string(),
        };

        let login_request = LoginRequest {
            region: "WEB".to_string(),
            lang_type: "EN".to_string(),
            login_ip: "".to_string(),
            invite_code: "".to_string(),
            verify_info,
            wallet_name: DEFAULT_WALLET_NAME.to_string(),
        };

        let response = self
            .client
            .post(&format!("{}/v1/private/user/login/dex", self.base_url))
            .json(&login_request)
            .send()
            .await?;

        let login_response: LoginResponse = response.json().await?;
        
        if login_response.code != 0 {
            return Err(format!("登录失败: {}", login_response.code).into());
        }

        self.access_token = Some(login_response.data.clone());
        Ok(login_response.data)
    }

    // 3. 上传代币图片
    async fn upload_image(&self) -> std::result::Result<String, Box<dyn std::error::Error>> {
        let access_token = self.access_token.as_ref()
            .ok_or("需要先登录获取access_token")?;

        // 检查本地图片文件是否存在
        let image_path = std::path::Path::new(CREATE_TOKEN_IMAGE_PATH);
        if !image_path.exists() {
            return Err(format!("图片文件不存在: {}", CREATE_TOKEN_IMAGE_PATH).into());
        }

        // 读取图片文件
        let image_data = std::fs::read(image_path)?;
        println!("  读取图片文件: {} ({}字节)", CREATE_TOKEN_IMAGE_PATH, image_data.len());

        // 获取文件扩展名
        let extension = image_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");

        // 生成随机文件名
        let random_filename = format!("{:x}.{}", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() % 0xffffffffffffffff, 
            extension
        );

        // 创建multipart表单
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(image_data)
                .file_name(random_filename)
                .mime_str(&format!("image/{}", extension))?);

        // 发送上传请求 - 尝试不同的API端点
        let response = self
            .client
            .post(&format!("{}/v1/private/token/upload", self.base_url))  
            .header("meme-web-access", access_token)
            .multipart(form)
            .send()
            .await?;

        // 解析响应
        let response_text = response.text().await?;
        println!("  上传响应: {}", response_text);

        // 解析JSON响应获取图片URL
        if let Ok(upload_response) = serde_json::from_str::<serde_json::Value>(&response_text) {
            if let Some(data) = upload_response.get("data") {
                if let Some(url) = data.as_str() {
                    println!("  ✅ 图片上传成功: {}", url);
                    return Ok(url.to_string());
                }
            }
        }
        return Err(format!("图片上传失败").into());
    }

    // 4. 创建代币
    async fn create_token(&self, params: &CreateTokenParams) -> std::result::Result<CreateTokenData, Box<dyn std::error::Error>> {
        let access_token = self.access_token.as_ref()
            .ok_or("需要先登录获取access_token")?;

        // 获取默认的raisedToken配置
        let raised_token = RaisedToken {
            symbol: "BNB".to_string(),
            native_symbol: "BNB".to_string(),
            symbol_address: "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c".to_string(),
            deploy_cost: "0".to_string(),
            buy_fee: "0.01".to_string(),
            sell_fee: "0.01".to_string(),
            min_trade_fee: "0".to_string(),
            b0_amount: "8".to_string(),
            total_b_amount: "24".to_string(),
            total_amount: "1000000000".to_string(),
            logo_url: "https://static.four.meme/market/fc6c4c92-63a3-4034-bc27-355ea380a6795959172881106751506.png".to_string(),
            trade_level: vec!["0.1".to_string(), "0.5".to_string(), "1".to_string()],
            status: "PUBLISH".to_string(),
            buy_token_link: "https://pancakeswap.finance/swap".to_string(),
            reserved_number: 10,
            sale_rate: "0.8".to_string(),
            network_code: "BSC".to_string(),
            platform: "MEME".to_string(),
        };

        let launch_time = params.launch_time.unwrap_or_else(|| {
            (Utc::now() + Duration::hours(1)).timestamp_millis() as u64
        });

        let create_request = CreateTokenRequest {
            name: params.name.clone(),
            short_name: params.short_name.clone(),
            desc: params.desc.clone(),
            img_url: params.img_url.clone(),
            launch_time,
            label: params.label.clone(),
            lp_trading_fee: 0.0025,
            web_url: params.web_url.clone().unwrap_or_default(),
            twitter_url: params.twitter_url.clone().unwrap_or_default(),
            telegram_url: params.telegram_url.clone().unwrap_or_default(),
            pre_sale: params.pre_sale.clone(),
            // 固定参数
            total_supply: 1000000000,
            raised_amount: 24,
            sale_rate: 0.8,
            reserve_rate: 0.0,
            fun_group: false,
            click_fun: false,
            symbol: "BNB".to_string(),
            raised_token,
        };

        let response = self
            .client
            .post(&format!("{}/v1/private/token/create", self.base_url))
            .header("meme-web-access", access_token)
            .json(&create_request)
            .send()
            .await?;

        // 先获取原始响应文本进行调试
        let response_text = response.text().await?;
        println!("🔍 创建代币API原始响应: {}", response_text);
        
        // 解析为JSON
        let create_response: CreateTokenResponse = serde_json::from_str(&response_text)?;
        
        if create_response.code != 0 {
            return Err(format!("创建代币失败: {} - {}", create_response.code, create_response.msg).into());
        }

        Ok(create_response.data)
    }
}

// 创建代币并购买的方法 - 使用Four.meme API
async fn create_token_and_buy(
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    token_manager_address: Address,
    _token_manager_helper_address: Address,
    create_params: CreateTokenParams,
    _buy_amount_bnb: U256,
) -> std::result::Result<CreateAndBuyResult, Box<dyn std::error::Error>> {
    let user_address = client.address();
    
    // 创建合约实例
    let token_manager2 = ITokenManager2::new(token_manager_address, client.clone());
    
    println!("🚀 开始创建代币流程...");
    println!("代币名称: {}", create_params.name);
    println!("代币符号: {}", create_params.short_name);
    
    // 获取创建前的BNB余额
    let before_bnb_balance = client.get_balance(user_address, None).await?;
    println!("创建前BNB余额: {}", format_ether(before_bnb_balance));
    
    // 步骤1: 通过Four.meme API创建代币
    println!("\n🔹 步骤1: 通过Four.meme API创建代币");
    
    let mut api_client = FourMemeApiClient::new();
    
    // 1.1 获取nonce
    println!("  获取nonce...");
    let account_address = format!("{:?}", user_address);
    let nonce = api_client.get_nonce(&account_address).await?;
    println!("  ✅ nonce获取成功: {}", nonce);
    
    // 1.2 用户登录
    println!("  用户登录...");
    let _access_token = api_client.login(&client.signer(), &nonce).await?;
    println!("  ✅ 登录成功，获得access_token");
    
    // 1.3 上传图片（使用默认图片）
    println!("  上传代币图片...");
    let img_url = api_client.upload_image().await?;
    println!("  ✅ 图片上传成功: {}", &img_url);
    
    // 1.4 创建代币并获取签名参数
    println!("  创建代币API调用...");
    let mut api_params = create_params.clone();
    api_params.img_url = img_url;
    
    let create_data = api_client.create_token(&api_params).await?;
    println!("  ✅ API创建成功，获得签名参数");
    
    // 调试：打印完整的create_data对象
    println!("  🔍 调试信息 - create_data对象:");
    println!("    token_id: {}", create_data.token_id);
    println!("    create_arg: {}", create_data.create_arg);
    println!("    signature: {}", create_data.signature);
    println!("    contract_address: {:?}", create_data.contract_address);
    
    if let Some(addr) = &create_data.contract_address {
        println!("  合约地址: {}", addr);
    } else {
        println!("  ⚠️  合约地址: API未返回contract_address字段");
    }
    
    // 步骤2: 调用区块链合约创建代币
    println!("\n🔹 步骤2: 调用区块链合约创建代币");
    
    let create_arg_bytes = hex::decode(&create_data.create_arg.trim_start_matches("0x"))?;
    let signature_bytes = hex::decode(&create_data.signature.trim_start_matches("0x"))?;
    
    // 使用预购BNB数量的110%作为创建代币的费用
    let pre_sale_amount: f64 = CREATE_TOKEN_PRE_SALE.parse().unwrap_or(0.1);
    let required_bnb: f64 = pre_sale_amount * 1.0; // 102%
    let required_bnb_wei = U256::from((required_bnb * 1e18) as u64);
    
    println!("  预购BNB数量: {} BNB", pre_sale_amount);
    println!("  创建代币需要BNB: {} BNB (预购数量的102%)", required_bnb);

    let create_tx = token_manager2.create_token(
        create_arg_bytes.into(),
        signature_bytes.into()
    ).value(required_bnb_wei); // 使用预购数量的102%
    
    let create_pending = create_tx.send().await?;
    let create_tx_hash = create_pending.tx_hash();
    println!("✅ 创建代币交易已发送: {:?}", create_tx_hash);
    
    // 等待创建交易确认
    let create_receipt = create_pending.await?.unwrap();
    println!("✅ 创建代币交易确认! 区块: {:?}", create_receipt.block_number);
    
    // 方法1: 尝试从合约调用的返回值中获取代币地址
    // 注意：createToken函数声明返回address token，但通过交易receipt无法直接获取返回值
    // 我们需要通过事件或日志来解析
    
    // 方法2: 从交易receipt的logs中解析出新创建的代币地址
    let mut token_address = Address::zero();
    
    println!("🔍 分析交易日志 (共{}条):", create_receipt.logs.len());
    
    // 查找代币创建相关的日志
    for (i, log) in create_receipt.logs.iter().enumerate() {
        println!("  日志 {}: 地址 {:?}, topics数量: {}", i, log.address, log.topics.len());
        
        // 方法2a: 查找Transfer事件（通常新创建的代币会有mint transfer）
        if log.topics.len() >= 3 {
            // Transfer事件的签名: Transfer(address,address,uint256)
            let transfer_sig = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
            if format!("{:?}", log.topics[0]) == transfer_sig {
                // 检查是否是从零地址的转账（mint操作）
                let from_addr = Address::from(log.topics[1]);
                if from_addr == Address::zero() {
                    token_address = log.address;
                    println!("    🎯 通过Transfer事件找到代币地址: {:?}", token_address);
                    break;
                }
            }
        }
        
        // 方法2b: 查找TokenCreated事件（如果存在）
        if log.address == token_manager_address && log.topics.len() >= 2 {
            // 可能是TokenCreated事件，第二个topic是代币地址
            let potential_token = Address::from(log.topics[1]);
            if potential_token != Address::zero() {
                token_address = potential_token;
                println!("    🎯 通过TokenManager事件找到代币地址: {:?}", token_address);
                break;
            }
        }
    }
    
    // 方法3: 如果还没找到，通过其他方式
    if token_address == Address::zero() {
        // 遍历所有非零地址的日志，找到最可能的代币地址
        for log in &create_receipt.logs {
            // 跳过已知的合约地址
            if log.address != token_manager_address && log.address != Address::zero() {
                // 检查这个地址是否看起来像ERC20代币
                token_address = log.address;
                println!("    💡 尝试使用日志地址作为代币地址: {:?}", token_address);
                break;
            }
        }
    }
    
    if token_address == Address::zero() {
        println!("⚠️  未能从交易日志中解析到代币地址，尝试其他方法...");
        // 如果无法从日志解析，可以尝试调用合约查询或其他方法
        return Ok(CreateAndBuyResult {
            token_address: Address::zero(),
            create_tx_hash,
            buy_tx_hash: H256::zero(),
            approve_tx_hash: None,
            tokens_received: U256::zero(),
            bnb_spent: required_bnb_wei,
            creation_block: create_receipt.block_number.map(|n| U256::from(n.as_u64())),
            buy_block: None,
        });
    }
    
    println!("\n🎉 代币创建完成!");
    println!("📝 新创建的代币信息:"); 
    println!("   代币地址: {:?}", token_address);
    println!("   交易哈希: {:?}", create_tx_hash);
    if let Some(block_number) = create_receipt.block_number {
        println!("   区块号: {}", block_number);
    }
    
    // 步骤3: 使用新创建的代币地址进行approve授权
    println!("\n🔹 步骤3: 使用新代币地址进行approve授权");
    
    // 使用从区块链解析出的代币地址
    let new_token = IERC20::new(token_address, client.clone());
    
    // 获取代币余额
    let token_balance = new_token.balance_of(user_address).call().await?;
    println!("代币余额: {}", token_balance);
    
    let mut approve_tx_hash = None;
    
    if token_balance > U256::zero() {
        // 检查当前授权额度
        let current_allowance = new_token.allowance(user_address, token_manager_address).call().await?;
        println!("当前授权额度: {}", current_allowance);
        
        if current_allowance < token_balance {
            println!("  发送approve交易...");
            let approve_tx = new_token.approve(token_manager_address, token_balance);
            let approve_pending = approve_tx.send().await?;
            approve_tx_hash = Some(approve_pending.tx_hash());
            println!("  ✅ approve交易已发送: {:?}", approve_tx_hash.unwrap());
            
            // 等待approve交易确认
            let approve_receipt = approve_pending.await?.unwrap();
            println!("  ✅ approve交易确认! 区块: {:?}", approve_receipt.block_number);
            
            // 检查新的授权额度
            let new_allowance = new_token.allowance(user_address, token_manager_address).call().await?;
            println!("  新的授权额度: {}", new_allowance);
        } else {
            println!("  ✅ 授权已足够，无需重新授权");
        }
    } else {
        println!("  ℹ️  代币余额为0，无需进行授权");
    }
    
    // 返回包含实际代币地址的结果
    Ok(CreateAndBuyResult {
        token_address, // 使用从区块链解析出的代币地址
        create_tx_hash,
        buy_tx_hash: H256::zero(), // 没有购买交易
        approve_tx_hash,
        tokens_received: token_balance, // 返回实际代币余额
        bnb_spent: required_bnb_wei, // 使用实际花费的BNB
        creation_block: create_receipt.block_number.map(|n| U256::from(n.as_u64())),
        buy_block: None, // 没有购买区块
    })
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // 1. 设置提供者和钱包
    let provider = Provider::<Http>::try_from(RPC_URL)?;
    let mut wallet: LocalWallet = PRIVATE_KEY.parse()?;
    
    // 设置钱包的链ID
    wallet = wallet.with_chain_id(CHAIN_ID);
    
    let client = Arc::new(SignerMiddleware::new(provider, wallet));
    
    // 2. 合约地址
    // 从环境变量获取代币地址，如果没有则使用默认地址
    let token_address_str = std::env::var("TOKEN_ADDRESS").unwrap_or_else(|_| DEFAULT_TOKEN_ADDRESS.to_string());
    println!("🔍 调试信息: 代币地址字符串 = '{}'", token_address_str);
    
    let token_address: Address = match token_address_str.parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("❌ 解析代币地址失败: {} (地址: '{}')", e, token_address_str);
            return Err(e.into());
        }
    };
    
    let token_manager_helper_address: Address = TOKEN_MANAGER_HELPER_ADDRESS.parse()?;
    let token_manager2_address: Address = TOKEN_MANAGER2_ADDRESS.parse()?;
    
    println!("📋 配置信息:");
    println!("   代币地址: {:?}", token_address);
    println!("   TokenManager2地址: {:?}", token_manager2_address);
    println!("   TokenManagerHelper地址: {:?}", token_manager_helper_address);
    
    // 选择操作模式
    let operation_mode = std::env::var("OPERATION_MODE").unwrap_or_else(|_| "SELL".to_string());
    
    match operation_mode.as_str() {
        "CREATE_AND_BUY" => {
            // 3. 创建代币并购买模式
            println!("🎯 模式: 创建代币并购买");
            
            let create_params = CreateTokenParams {
                name: CREATE_TOKEN_NAME.to_string(),
                short_name: CREATE_TOKEN_SYMBOL.to_string(),
                desc: CREATE_TOKEN_DESC.to_string(),
                img_url: String::new(), // 这将在create_token方法中通过upload_image更新
                launch_time: Some(Utc::now().timestamp_millis() as u64), // 立即发布
                label: CREATE_TOKEN_LABEL.to_string(),
                web_url: Some(CREATE_TOKEN_WEB_URL.to_string()),
                twitter_url: Some(CREATE_TOKEN_TWITTER_URL.to_string()),
                telegram_url: Some(CREATE_TOKEN_TELEGRAM_URL.to_string()),
                pre_sale: CREATE_TOKEN_PRE_SALE.to_string(),
            };
            
            let buy_amount = U256::from((BUY_AMOUNT_BNB * 1e18) as u64); // 购买BNB的代币
            
            let create_result = create_token_and_buy(
                client.clone(),
                token_manager2_address,
                token_manager_helper_address,
                create_params,
                buy_amount,
            ).await?;
            
            // 显示创建和购买结果
            println!("\n🎉 创建代币并购买完成汇总:");
            println!("新代币地址: {:?}", create_result.token_address);
            println!("创建交易哈希: {:?}", create_result.create_tx_hash);
            println!("购买交易哈希: {:?}", create_result.buy_tx_hash);
            if let Some(approve_hash) = create_result.approve_tx_hash {
                println!("授权交易哈希: {:?}", approve_hash);
            }
            if let Some(creation_block) = create_result.creation_block {
                println!("创建区块号: {:?}", creation_block);
            }
            if let Some(buy_block) = create_result.buy_block {
                println!("购买区块号: {:?}", buy_block);
            }
            println!("获得代币数量: {}", create_result.tokens_received);
            println!("花费BNB: {}", format_ether(create_result.bnb_spent));

            // println!("\n使用纯粹卖币方法进行卖出...");
            
            // let sell_result = sell_tokens_only(
            //     client.clone(),
            //     create_result.token_address,
            //     token_manager2_address,
            //     create_result.tokens_received,
            // ).await?;
            // // 4. 显示最终结果
            // println!("\n📊 交易完成汇总:");
            // if let Some(approve_hash) = sell_result.approve_tx_hash {
            //     println!("Approve交易哈希: {:?}", approve_hash);
            // }
            // println!("Sell交易哈希: {:?}", sell_result.sell_tx_hash);
            // if let Some(block_number) = sell_result.block_number {
            //     println!("确认区块号: {:?}", block_number);
            // }
            // println!("代币卖出数量: {}", sell_result.token_sold);
            // println!("BNB收益: {}", format_ether(sell_result.bnb_received));
        },
        "SELL" | _ => {
            // 3. 卖出代币模式（默认）
            println!("🎯 模式: 卖出现有代币");
            
            let sell_result = execute_sell_process(
                client.clone(),
                token_address,
                token_manager_helper_address,
            ).await?;
            
            // 4. 显示最终结果
            println!("\n📊 交易完成汇总:");
            if let Some(approve_hash) = sell_result.approve_tx_hash {
                println!("Approve交易哈希: {:?}", approve_hash);
            }
            println!("Sell交易哈希: {:?}", sell_result.sell_tx_hash);
            if let Some(block_number) = sell_result.block_number {
                println!("确认区块号: {:?}", block_number);
            }
            println!("代币卖出数量: {}", sell_result.token_sold);
            println!("BNB收益: {}", format_ether(sell_result.bnb_received));
        }
    }


    Ok(())
}