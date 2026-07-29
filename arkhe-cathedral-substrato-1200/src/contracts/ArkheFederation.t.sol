// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./ArkheFederation.sol";

contract ArkheFederationTest {
    ArkheFederation federation;

    function setUp() public {
        federation = new ArkheFederation();
    }

    function testJoin() public {
        federation.join();
    }

    function testHeartbeat() public {
        federation.heartbeat();
    }

    function testRouteTask() public {
        federation.routeTask();
    }

    function testVerifyTask() public {
        federation.verifyTask();
    }

    function testSlash() public {
        federation.slash();
    }
}
