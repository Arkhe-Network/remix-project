# Taproot Assets gRPC API Mapping

## Architecture Overview
- `tapd` (Taproot Assets Daemon) runs alongside `LND`.
- Communicates via gRPC (10029) and REST (8089).
- Uses Macaroons and TLS for authentication.

## Services

### 1. `TaprootAssets` (Core)
- **GetInfo**: Node information.
- **ListAssets**: Lists all assets.
- **ListBalances**: Balances per asset.
- **ListGroups**: Known asset groups.
- **ListTransfers**: Tracked transfers.
- **NewAddr**: Generates receiving address.
- **SendAsset**: Sends asset to address.
- **BurnAsset**: Burns asset units.
- **VerifyProof**: Verifies an asset proof file.

### 2. `AssetWallet`
- **CreateAsset**: Mints a new asset.
- **IssueAsset**: Issues more of an existing asset.
- **TransferAsset**: Advanced asset transfer.

### 3. `Universe`
- **QueryUniverse**: Queries asset proofs.
- **SyncUniverse**: Syncs with other Universe servers.
