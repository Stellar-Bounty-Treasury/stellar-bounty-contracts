const fs = require('fs');
const path = require('path');
const {
  Keypair,
  rpc,
  TransactionBuilder,
  Operation,
  Networks,
  Address,
} = require('@stellar/stellar-sdk');

async function deploy() {
  const secret = 'SAJOW7MR22FBJYEMS3NP7IMSQ2ZMSVUBAAGBY2Y6EOOLQSEVQ5FFEC6E';
  const deployer = Keypair.fromSecret(secret);
  const server = new rpc.Server('https://soroban-testnet.stellar.org');

  console.log('Deployer Public Key:', deployer.publicKey());

  const wasmHashBuffer = Buffer.from(
    '48e83035a07f93569ebe473bb4c0d74ee91830e222b33d3fea1f22a33d4b1db7',
    'hex'
  );

  console.log('--- Step 2: Creating Contract Instance from Uploaded WASM ---');
  let account = await server.getAccount(deployer.publicKey());

  let createTx = new TransactionBuilder(account, {
    fee: '1000000',
    networkPassphrase: Networks.TESTNET,
  })
    .addOperation(
      Operation.createCustomContract({
        address: Address.fromString(deployer.publicKey()),
        wasmHash: wasmHashBuffer,
      })
    )
    .setTimeout(300)
    .build();

  console.log('Simulating contract creation transaction...');
  createTx = await server.prepareTransaction(createTx);
  createTx.sign(deployer);

  const sendCreateRes = await server.sendTransaction(createTx);
  console.log('Create Contract Tx Hash:', sendCreateRes.hash);

  let getCreateRes = await server.getTransaction(sendCreateRes.hash);
  while (getCreateRes.status === 'NOT_FOUND') {
    await new Promise((r) => setTimeout(r, 2000));
    getCreateRes = await server.getTransaction(sendCreateRes.hash);
  }

  if (getCreateRes.status !== 'SUCCESS') {
    throw new Error(`Create contract failed on-chain: ${JSON.stringify(getCreateRes)}`);
  }

  const contractAddress = Address.fromScAddress(getCreateRes.returnValue).toString();
  console.log('====================================================');
  console.log('🎉 Soroban Bounty Treasury Contract Deployed to Stellar Testnet!');
  console.log('Contract Address:', contractAddress);
  console.log('Deployment Tx Hash:', sendCreateRes.hash);
  console.log('Explorer:', `https://stellar.expert/explorer/testnet/contract/${contractAddress}`);
  console.log('====================================================');

  // Save deployment info
  fs.writeFileSync(
    path.resolve(__dirname, 'deployment.json'),
    JSON.stringify(
      {
        network: 'testnet',
        contractAddress,
        wasmHash: '48e83035a07f93569ebe473bb4c0d74ee91830e222b33d3fea1f22a33d4b1db7',
        deployer: deployer.publicKey(),
        uploadTxHash: '9fb1729caeb1772db3a31c5d32bc5ff11fe68afd0705643bc8cfcbe8d52165b0',
        createTxHash: sendCreateRes.hash,
        deployedAt: new Date().toISOString(),
      },
      null,
      2
    )
  );
}

deploy().catch(console.error);
