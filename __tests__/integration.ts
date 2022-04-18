import { fromUtf8 } from "@cosmjs/encoding";
import fs from "fs";
import util from "util";
import {
  MsgExecuteContract,
  SecretNetworkClient,
  Wallet,
} from "./src";
import { AminoWallet } from "./src/wallet_amino";

const exec = util.promisify(require("child_process").exec);

type Result = {
  status: string;
  result: string[];
};

type Account = {
  name: string;
  type: string;
  address: string;
  pubkey: string;
  mnemonic: string;
  walletAmino: AminoWallet;
  walletProto: Wallet;
  secretjs: SecretNetworkClient;
};

const accounts: Account[] = [];

async function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function getMnemonicRegexForAccountName(account: string) {
  return new RegExp(`{"name":"${account}".+?"mnemonic":".+?"}`);
}

function getValueFromRawLog(rawLog: string | undefined, key: string): string {
  if (!rawLog) {
    return "";
  }

  for (const l of JSON.parse(rawLog)) {
    for (const e of l.events) {
      for (const a of e.attributes) {
        if (`${e.type}.${a.key}` === key) {
          return String(a.value);
        }
      }
    }
  }

  return "";
}

async function performCreateViewingKey(secretjs: SecretNetworkClient, contractAddress: string) {
  const txExec = await secretjs.tx.compute.executeContract(
    {
      sender: accounts[0].address,
      contract: contractAddress,
      // codeHash,
      msg: {
        create_viewing_key: {
          entropy: "bla bla",
        },
      },
    },
    {
      gasLimit: 5000000,
    }
  );
  expect(fromUtf8(txExec.data[0])).toContain(
    '{"create_viewing_key":{"key":"'
  );
  const viewingKey = JSON.parse(fromUtf8(txExec.data[0])).create_viewing_key.key;
  return viewingKey;
}

async function performSomething(contractAddress: string, secretjs: SecretNetworkClient, operation: string, param: string) {
  let addMsg: MsgExecuteContract;

    addMsg = new MsgExecuteContract({
      sender: accounts[0].address,
      contract: contractAddress,
      // codeHash, // Test MsgExecuteContract without codeHash
      msg: { [operation]: param },
      sentFunds: [],
    });

  const tx = await secretjs.tx.broadcast([addMsg], {
    gasLimit: 5000000,
  });

  expect(tx.code).toBe(0);
  expect(getValueFromRawLog(tx.rawLog, "message.action")).toBe("execute");
  expect(getValueFromRawLog(tx.rawLog, "wasm.contract_address")).toBe(
    contractAddress
  );
  // Check decryption
  expect(tx.arrayLog![4].key).toBe("contract_address");
  expect(tx.arrayLog![4].value).toBe(contractAddress);
}

beforeAll(async () => {
  try {
    // init testnet
    console.log("Setting up a local testnet...");
    await exec("docker rm -f secretjs-testnet || true");
    const { /* stdout, */ stderr } = await exec(
      "docker run -it -d -p 9091:9091 --name secretjs-testnet enigmampc/secret-network-sw-dev:v1.2.2-1",
    );
    // console.log("stdout (testnet container id?):", stdout);
    if (stderr) {
      console.error("stderr:", stderr);
    }

    // Wait for the network to start (i.e. block number >= 1)
    console.log("Waiting for the network to start...");

    const timeout = Date.now() + 30_000;
    while (true) {
      expect(Date.now()).toBeLessThan(timeout);

      const secretjs = await SecretNetworkClient.create({
        grpcWebUrl: "http://localhost:9091",
        chainId: "secretdev-1",
      });

      try {
        const { block } = await secretjs.query.tendermint.getLatestBlock({});

        if (Number(block?.header?.height) >= 1) {
          break;
        }
      } catch (e) {
        // console.error(e);
      }
      await sleep(250);
    }

    // Extract genesis accounts from logs
    const accountIdToName = ["a", "b", "c", "d"];
    const { stdout: dockerLogsStdout } = await exec(
      "docker logs secretjs-testnet",
    );
    const logs = String(dockerLogsStdout);
    for (const accountId of [0, 1, 2, 3]) {
      if (!accounts[accountId]) {
        const match = logs.match(
          getMnemonicRegexForAccountName(accountIdToName[accountId]),
        );
        if (match) {
          const parsedAccount = JSON.parse(match[0]) as Account;
          parsedAccount.walletAmino = new AminoWallet(parsedAccount.mnemonic);
          parsedAccount.walletProto = new Wallet(parsedAccount.mnemonic);
          parsedAccount.secretjs = await SecretNetworkClient.create({
            grpcWebUrl: "http://localhost:9091",
            chainId: "secretdev-1",
            wallet: parsedAccount.walletAmino,
            walletAddress: parsedAccount.address,
          });
          accounts[accountId] = parsedAccount as Account;
        }
      }
    }
  } catch (e) {
    console.error("Setup failed:", e);
  }
}, 45_000);

afterAll(async () => {
  try {
    console.log("Tearing down local testnet...");
    const { stdout, stderr } = await exec("docker rm -f secretjs-testnet");
    // console.log("stdout (testnet container name?):", stdout);
    if (stderr) {
      console.error("stderr:", stderr);
    }
  } catch (e) {
    console.error("Teardown failed:", e);
  }
});

describe("tx.compute and query.compute", () => {
  let contractAddress: string;
  let contractCodeHash: string;
  let viewingKey: string;

  beforeAll(async () => {
    const { secretjs } = accounts[0];
    const txStore = await secretjs.tx.compute.storeCode(
      {
        sender: accounts[0].address,
        wasmByteCode: fs.readFileSync(
          `${__dirname}/../contract.wasm.gz`,
        ) as Uint8Array,
        source: "",
        builder: "",
      },
      {
        gasLimit: 5_000_000,
      },
    );

    expect(txStore.code).toBe(0);

    const codeId = Number(
      getValueFromRawLog(txStore.rawLog, "message.code_id"),
    );

    const {
      codeInfo: { codeHash },
    } = await secretjs.query.compute.code(codeId);
    contractCodeHash = codeHash

    const txInit = await secretjs.tx.compute.instantiateContract(
      {
        sender: accounts[0].address,
        codeId,
        // codeHash, // Test MsgInstantiateContract without codeHash
        initMsg: {
          prng_seed: "waehfjklasd",
        },
        label: `label-${Date.now()}`,
        initFunds: [],
      },
      {
        gasLimit: 5_000_000,
      },
    );

    expect(txInit.code).toBe(0);

    contractAddress = getValueFromRawLog(txInit.rawLog, "wasm.contract_address");

    viewingKey = await performCreateViewingKey(secretjs, contractAddress);
  });

  test("Perform ____", async () => {
    const { secretjs } = accounts[0];

    await performSomething(contractAddress, secretjs, "add", "2");

    const result = (await secretjs.query.compute.queryContract({
      address: contractAddress,
      codeHash: contractCodeHash,
      query: { get_history: {address: accounts[0].address, key: viewingKey, page_size: 1} },
    })) as Result;

    // expect(result).toStrictEqual(
    //   {
    //     status: "Calculations history present",
    //     result: ["2 + 3 = 5"],
    //   },
    // );
  });


});
