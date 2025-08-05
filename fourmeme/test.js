const { ethers } = require("hardhat");
const { expect } = require("chai");
require('dotenv').config();

describe("TokenTradeWithFee 部署测试", function () {
  let TokenTradeWithFee, tokenTrade, owner, feeRecipient,user;

  beforeEach(async function () {
    const privateKey = process.env.PRIVATE_KEY;
    const wallet = new ethers.Wallet(privateKey, ethers.provider);

    // 设置 owner 和 feeRecipient 为钱包和 Hardhat provider
    const signers = await ethers.getSigners();
    [owner, user,feeRecipient] = [wallet, signers[1],signers[2]];

    // 打印账户信息以确认
    console.log("Owner 地址:", owner.address);
    console.log("FeeRecipient 地址:", feeRecipient.address);
    
    const tokenManagerHelperAddress = "0xF251F83e40a78868FcfA3FA4599Dad6494E46034";

    // 部署合约
    try {
      console.log("开始部署 TokenTradeWithFee...");
      console.log("feeRecipient.address:", feeRecipient.address);
      TokenTradeWithFee = await ethers.getContractFactory("TokenTradeWithFee",owner);
      // 部署合约
      tokenTrade = await TokenTradeWithFee.deploy(
        tokenManagerHelperAddress,
        feeRecipient.address,
        1, 
      );
      // 等待交易确认
      await tokenTrade.waitForDeployment();
      console.log("合约成功部署，地址:", tokenTrade.target);
    } catch (error) {
      console.error("部署过程中出错:", error);
      if (error.reason) {
        console.error("回滚原因:", error.reason);
      }
      throw error; // 抛出错误以使测试失败
    }
  });

  it("应正确设置费用和费用接收者", async function () {
    let feeAddress= await tokenTrade.feeRecipient();
    console.log("fee",feeAddress);
    expect(feeAddress).to.equal(feeRecipient.address);
    console.log("费用接收者地址已设置:", await tokenTrade.feeRecipient());
    let fee=await tokenTrade.feeRate();
    expect(fee).to.equal(1);
    console.log("费用已设置:", await tokenTrade.feeRate());
  });

  it("费用设置不能超过5%",async function(){
    const invalidFeeRate = 6;
    await expect(
      tokenTrade.connect(owner).setFee(invalidFeeRate, feeRecipient.address)
    ).to.be.revertedWith("FR - FeeRate exceeds 5%");
  });
  
  it("不应允许费用接收者为零地址", async function () {
    const newFeeRate = 1;
    const invalidFeeRecipient = ethers.ZeroAddress;
    await expect(
      tokenTrade.connect(owner).setFee(newFeeRate, invalidFeeRecipient)
    ).to.be.revertedWith("ZA - Zero Address");
  });

  it("只有owner可以设置费用", async function () {
    const newFeeRate = 3;
    const nonOwner = (await ethers.getSigners())[2];
    await expect(
      tokenTrade.connect(nonOwner).setFee(newFeeRate, feeRecipient.address)
    ).to.be.revertedWith("Only the owner can perform this action");
  });

  it("获取token Info", async function () {
    const tokenAddress="0xa8bd669db5b3eceea995a26208aca6812fa22901";
    const result = await tokenTrade.connect(user).getTokenInfo(tokenAddress);
    console.log(result);
  });

  it("仅支持BNB交易对代币", async function () {
    const funds = ethers.parseEther("1.01");
    const tokenAddress="0xd7de45c6af7b67d222f5e2af35f8cd3285534444";
    await expect(
      tokenTrade.connect(user).buyTokenWithFee(tokenAddress,0,  { value: funds })
    )
    .to.rejectedWith("Only BNB trading pairs are supported");
  });

  it("仅支持未发射到pancake的代币", async function () {
    const funds = ethers.parseEther("1.01");
    const tokenAddress="0x9d925624b4b322283ee20fd5bd28b7e66bbe4444";
    const result = await tokenTrade.connect(user).getTokenInfo(tokenAddress);
    console.log(result);
    await expect(
      tokenTrade.connect(user).buyTokenWithFee(tokenAddress,0, { value: funds })
    )
    .to.rejectedWith("Token must NOT be listed on PancakeSwap");
  });

  // it("仅支持Manager version 2 代币", async function () {
  //   const funds = ethers.parseEther("1.01");
  //   const tokenAddress="0xc144587024d3a08943ff556056d54303a7134444";
  //   const result = await tokenTrade.connect(user).getTokenInfo(tokenAddress);
  //   console.log(result);
  //   await expect(
  //     tokenTrade.connect(user).buyTokenWithFee(tokenAddress, 0, { value: funds })
  //   )
  //   .to.rejectedWith("Invalid TokenManager version");
  // });

  it("应正确处理BNB购买代币交易", async function () {
    //8000000000
    const tokenAmount = 0;
    const bnbAmount = ethers.parseEther("0.0001");
    const funds = ethers.parseEther("1.01");
    const tokenAddress="0x7ead2c9072b4a40853f13f06e481ce71c5874444";

    const token = await ethers.getContractAt("IERC20", tokenAddress);
    const beforeTokenBalance = await token.balanceOf(user.address);
    console.log("用户初始代币余额:", ethers.formatUnits(beforeTokenBalance, 18));

    // 获取用户的初始BNB余额
    const initialBalance = await ethers.provider.getBalance(user.address);
    console.log("用户初始BNB余额:", ethers.formatEther(initialBalance));

    const result2=await tokenTrade.connect(user).tryBuy(tokenAddress, tokenAmount, bnbAmount);
    console.log("restul:",result2);
    
    // 执行购买操作
    const minAmount = result2.estimatedAmount;
    const tx = await tokenTrade.connect(user).buyTokenWithFee(tokenAddress,minAmount, { value: funds });
    
    // 等待交易确认
    await tx.wait();

    const afterTokenBalance = await token.balanceOf(user.address);
    // 检查买到的 token 数量
    const boughtAmount = afterTokenBalance - beforeTokenBalance;
    console.log("买到的代币数量:", boughtAmount.toString());
    expect(boughtAmount).to.be.gte(minAmount);

    // 获取用户的最终BNB余额
    const finalBalance = await ethers.provider.getBalance(user.address);
    console.log("用户最终BNB余额:", ethers.formatEther(finalBalance));
    // 检查余额变化
    expect(finalBalance).to.be.lt(initialBalance);
  });


  it("应正确处理BNB买入并卖出代币交易", async function () {
      const IERC20_ABI = [
        "function allowance(address owner, address spender) view returns (uint256)",
        "function approve(address spender, uint256 amount) returns (bool)",
        "function balanceOf(address account) view returns (uint256)",
        "function transfer(address to, uint256 amount) returns (bool)",
        "function transferFrom(address from, address to, uint256 amount) returns (bool)"
      ];
      const tokenAddress = "0x2b863b6fc88ce451cd5b31a2c5c049975fb84444";

      
      const invalidFeeRate = 0;
      await expect(
        tokenTrade.connect(owner).setFee(invalidFeeRate, feeRecipient.address)
      ).to.be.revertedWith("FR - FeeRate exceeds 5%");
      
      const result = await tokenTrade.connect(user).getTokenInfo(tokenAddress);
      console.log(result);
      const token = await ethers.getContractAt(IERC20_ABI, tokenAddress);
      // 1. 买入
      const bnbAmount = ethers.parseEther("0.01");
      const funds = ethers.parseEther("0.02");
      const buyResult = await tokenTrade.connect(user).tryBuy(tokenAddress, 0, bnbAmount);
      console.log("buyResult.tokenManager:", buyResult.tokenManager);
      const minAmount = buyResult.estimatedAmount;
      // 买入前的 token 余额
      const beforeBuyTokenBalance = await token.balanceOf(user.address);
      // 买入
      const buyTx = await tokenTrade.connect(user).buyTokenWithFee(tokenAddress, minAmount, { value: funds });
      await buyTx.wait();
      // 买入后的 token 余额
      const afterBuyTokenBalance = await token.balanceOf(user.address);
      const boughtAmount = afterBuyTokenBalance - beforeBuyTokenBalance;
      expect(boughtAmount).to.be.gte(minAmount);
      console.log("买入的代币数量:", boughtAmount.toString());
      // 2. 卖出
      const sellAmount = boughtAmount; // 卖出刚买到的全部
      const initialBNBBalance = await ethers.provider.getBalance(user.address);
      // 预估能卖出多少BNB
      const sellResult = await tokenTrade.connect(user).trySell(tokenAddress, sellAmount);
      const minFunds = sellResult.funds;
      // 用户授权合约花费 token
      await token.connect(user).approve(buyResult.tokenManager, sellAmount);
      const allowance = await token.allowance(user.address, buyResult.tokenManager);
      console.log("授权额度:", allowance.toString());
      // 卖出
      const sellTx = await tokenTrade.connect(user).sellTokenWithFee(tokenAddress, sellAmount, minFunds);
      await sellTx.wait();
      // 卖出后的 token 余额
      const afterSellTokenBalance = await token.balanceOf(user.address);
      expect(afterSellTokenBalance).to.be.lt(afterBuyTokenBalance);
      // 卖出后的BNB余额
      const finalBNBBalance = await ethers.provider.getBalance(user.address);
      expect(finalBNBBalance).to.be.gt(initialBNBBalance);
  });


});