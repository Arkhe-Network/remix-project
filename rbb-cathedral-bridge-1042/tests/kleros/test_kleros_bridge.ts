import { expect } from "chai";
import { ethers } from "hardhat";

describe("Cathedral Kleros Bridge", function () {
  let owner: any;
  let messenger: any;
  let juror1: any;
  let juror2: any;

  let oracle: any;
  let bridge: any;

  beforeEach(async function () {
    [owner, messenger, juror1, juror2] = await ethers.getSigners();

    // Deploy Oracle
    const Oracle = await ethers.getContractFactory("PNKTheosisOracle");
    oracle = await Oracle.deploy();
    await oracle.waitForDeployment();

    // Authorize owner to update
    await oracle.addUpdater(owner.address);

    // Set some Theosis levels
    await oracle.updateTheosis(juror1.address, 2); // juror1 gets a theosis of 2
    await oracle.updateTheosis(juror2.address, 5); // juror2 gets a theosis of 5

    // Deploy Bridge with Voting
    const Bridge = await ethers.getContractFactory("CathedralKlerosBridgeWithVoting");
    bridge = await Bridge.deploy(messenger.address, await oracle.getAddress());
    await bridge.waitForDeployment();
  });

  it("Should allow a juror to cast a weighted vote based on Theosis", async function () {
    const disputeID = 1;
    const rulingChoiceJuror1 = 1; // e.g. "Yes"

    await expect(bridge.connect(juror1).castWeightedVote(disputeID, rulingChoiceJuror1))
      .to.emit(bridge, "VoteCast")
      .withArgs(disputeID, juror1.address, rulingChoiceJuror1, 3); // Base 1 + Theosis 2 = 3

    const votes = await bridge.weightedVotes(disputeID, rulingChoiceJuror1);
    expect(votes).to.equal(3);
  });

  it("Should resolve the dispute correctly taking weights into account", async function () {
    const disputeID = 2;

    // juror1 votes for ruling 1 (weight: 1+2=3)
    await bridge.connect(juror1).castWeightedVote(disputeID, 1);

    // juror2 votes for ruling 2 (weight: 1+5=6)
    await bridge.connect(juror2).castWeightedVote(disputeID, 2);

    // Resolve the dispute
    const rulingOptions = [1, 2];
    await expect(bridge.connect(messenger).resolveWeightedDispute(disputeID, rulingOptions))
      .to.emit(bridge, "DisputeResolvedWithWeightedVoting")
      .withArgs(disputeID, 2); // 2 should win because weight 6 > 3
  });

  it("Should prevent double voting", async function () {
    const disputeID = 3;
    await bridge.connect(juror1).castWeightedVote(disputeID, 1);

    await expect(bridge.connect(juror1).castWeightedVote(disputeID, 2))
      .to.be.revertedWith("Juror already voted");
  });
});
