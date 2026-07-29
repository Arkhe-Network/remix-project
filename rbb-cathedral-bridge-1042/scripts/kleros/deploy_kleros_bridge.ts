import { ethers } from "hardhat";

async function main() {
  console.log("Starting Hardhat Deployment for Kleros Bridge Contracts...");

  const [deployer] = await ethers.getSigners();
  console.log("Deploying contracts with the account:", deployer.address);

  // 1. Deploy PNKTheosisOracle
  const Oracle = await ethers.getContractFactory("PNKTheosisOracle");
  const oracle = await Oracle.deploy();
  await oracle.waitForDeployment();
  const oracleAddress = await oracle.getAddress();
  console.log("PNKTheosisOracle deployed to:", oracleAddress);

  // CrossChainMessenger mockup address for this example
  const mockMessenger = "0x0000000000000000000000000000000000000001";

  // 2. Deploy CathedralKlerosBridge
  const BaseBridge = await ethers.getContractFactory("CathedralKlerosBridge");
  const baseBridge = await BaseBridge.deploy(mockMessenger);
  await baseBridge.waitForDeployment();
  const baseBridgeAddress = await baseBridge.getAddress();
  console.log("CathedralKlerosBridge deployed to:", baseBridgeAddress);

  // 3. Deploy CathedralKlerosBridgeWithVoting
  const VotingBridge = await ethers.getContractFactory("CathedralKlerosBridgeWithVoting");
  const votingBridge = await VotingBridge.deploy(mockMessenger, oracleAddress);
  await votingBridge.waitForDeployment();
  const votingBridgeAddress = await votingBridge.getAddress();
  console.log("CathedralKlerosBridgeWithVoting deployed to:", votingBridgeAddress);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
