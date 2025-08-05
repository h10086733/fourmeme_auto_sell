// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
import "hardhat/console.sol";


import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

interface ITokenManager2 {
    function buyTokenAMAP(address token, address to, uint256 funds, uint256 minAmount) external payable ;
    function sellToken(uint256 origin, address token,  address from, uint256 amount, uint256 minFunds, uint256 feeRate, address feeRecipient) external;
}

interface ITokenManagerHelper3 {
    function getTokenInfo(address token) external view returns (
        uint256 version,
        address tokenManager,
        address quote,
        uint256 lastPrice,
        uint256 tradingFeeRate,
        uint256 minTradingFee,
        uint256 launchTime,
        uint256 offers,
        uint256 maxOffers,
        uint256 funds,
        uint256 maxFunds,
        bool liquidityAdded
    );
    function tryBuy(address token, uint256 amount, uint256 funds) external view returns (
        address tokenManager,
        address quote,
        uint256 estimatedAmount,
        uint256 estimatedCost,
        uint256 estimatedFee,
        uint256 amountMsgValue,
        uint256 amountApproval,
        uint256 amountFunds
    );
    function trySell(address token, uint256 amount) external view returns (
        address tokenManager,
        address quote,
        uint256 funds,
        uint256 fee
    );
    function buyWithEth(uint256 origin, address token, address to, uint256 funds, uint256 minAmount) external payable;
    function sellForEth(uint256 origin, address token, uint256 amount, uint256 minFunds, uint256 feeRate, address feeRecipient) external;
}

interface IERC20 {
    function approve(address spender, uint256 amount) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

contract TokenTradeWithFee is ReentrancyGuard {
    address public immutable owner;
    ITokenManagerHelper3 public immutable tokenManagerHelper;
    address public feeRecipient;
    uint256 public feeRate; // Fee rate in percentage (e.g., 1 means 1%)

    // Events for logging
    event ProxyTokenPurchase(address indexed token, address indexed account, uint256 tokenAmount, uint256 feeAmount);
    event ProxyTokenSold(address indexed token, uint256 amount, uint256 fundsReceived);
    event FeeSet(uint256 feeRate, address feeRecipient);

    // Constructor to set initial values
    constructor(address _tokenManagerHelper, address _feeRecipient, uint256 _feeRate) {
        require(_tokenManagerHelper != address(0), "ZA - Zero Address for helper");
        require(_feeRecipient != address(0), "ZA - Zero Address for fee recipient");
        require(_feeRate <= 5, "FR - FeeRate exceeds 5%");
        owner = msg.sender;
        tokenManagerHelper = ITokenManagerHelper3(_tokenManagerHelper);
        feeRecipient = _feeRecipient;
        feeRate = _feeRate;
    }

    // Modifier to restrict access to owner
    modifier onlyOwner() {
        require(msg.sender == owner, "Only the owner can perform this action");
        _;
    }

    // Function to set the fee and fee recipient
    function setFee(uint256 _feeRate, address _feeRecipient) external onlyOwner {
        require(_feeRate <= 5, "FR - FeeRate exceeds 5%");
        require(_feeRecipient != address(0), "ZA - Zero Address");
        feeRate = _feeRate;
        feeRecipient = _feeRecipient;
        emit FeeSet(_feeRate, _feeRecipient);
    }

    function tryBuy(address token, uint256 amount, uint256 funds) public view returns (
        address tokenManager,
        address quote,
        uint256 estimatedAmount,
        uint256 estimatedCost,
        uint256 estimatedFee,
        uint256 amountMsgValue,
        uint256 amountApproval,
        uint256 amountFunds
    ){
        return tokenManagerHelper.tryBuy(token, amount, funds);
    }

    
    function trySell(address token, uint256 amount) public view returns (
        address tokenManager,
        address quote,
        uint256 funds,
        uint256 fee
    ){
        return tokenManagerHelper.trySell(token, amount);
    }

    // Function to get token info
    function getTokenInfo(address token) public view returns (
        uint256 version,
        address tokenManager,
        address quote,
        uint256 lastPrice,
        uint256 tradingFeeRate,
        uint256 minTradingFee,
        uint256 launchTime,
        uint256 offers,
        uint256 maxOffers,
        uint256 funds,
        uint256 maxFunds,
        bool liquidityAdded
    ) {
        return tokenManagerHelper.getTokenInfo(token);
    }

    // Validate token data
    function validate(address token) internal view returns (address tokenManager) {
        (uint256 version, address _tokenManager, address quote, , , , , , , , , bool liquidityAdded) = tokenManagerHelper.getTokenInfo(token);
        require(quote == address(0), "Only BNB trading pairs are supported");
        require(!liquidityAdded, "Token must NOT be listed on PancakeSwap");
        require(version == 2, "Invalid TokenManager version");
        return _tokenManager;
    }

    function buyTokenWithFee(address token,uint256 minAmount) external payable nonReentrant {
        address tokenManager = validate(token);
        // 计算手续费
        uint256 fee = (msg.value * feeRate) / 100;
        require(msg.value > fee, "Insufficient BNB for fee");
        uint256 fundsAfterFee = msg.value - fee;

        // 预估能买多少token
        (
            ,
            ,uint256 estimatedAmount
            ,
            ,
            ,
            uint256 amountMsgValue,
            ,
            uint256 amountFunds
        ) = tokenManagerHelper.tryBuy(token, 0, fundsAfterFee);

        require(estimatedAmount >= minAmount, "Slippage: insufficient output");

        // 转手续费
        payable(feeRecipient).transfer(fee);

        // 买币
        ITokenManager2(tokenManager).buyTokenAMAP{value: amountMsgValue}(token, msg.sender, amountFunds, minAmount);

        emit ProxyTokenPurchase(token, msg.sender, amountMsgValue, fee);
    }
    // Function to sell tokens and deduct fee
    function sellTokenWithFee(address token, uint256 amount, uint256 minFunds) external nonReentrant {
        address tokenManager = validate(token);
        uint256 allowance = IERC20(token).allowance(msg.sender,tokenManager);
        require(allowance >= amount, "Insufficient allowance");
        ITokenManager2(tokenManager).sellToken(0, token, msg.sender ,amount, minFunds, feeRate*100, feeRecipient);
        emit ProxyTokenSold(token,amount, minFunds);
    }

    // Fallback function to receive BNB
    receive() external payable {}
}
