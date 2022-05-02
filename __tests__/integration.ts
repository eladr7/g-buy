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


type StaticItemData = {
  name: string,
  category: string, // Elad: Might be redundant.
  url: string,
  img_url: string,
  seller_address: string,
  seller_email: string,
  price: string,
  wanted_price: string,
  group_size_goal: number,
} 

type ItemData = {
  static_data: StaticItemData,
  current_group_size: number,
}

type UserProductQuantity = {
  url: string,
  quantity: number,
}

type UserContactData = {
  email: string,
  delivery_address: string,
}
type Result = {
  items: ItemData[];
  user_items: UserProductQuantity[];
  contact_data?: UserContactData;
  status: string;
};

type UserItemDetails = {
  account_address: string,
  contact_data: UserContactData,
  quantity: number,
}

type UpdateItemData = {
  category: string,
  url: string,
  user_details: UserItemDetails,
}

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

async function performSetViewingKey(secretjs: SecretNetworkClient, contractAddress: string, viewingKey: string) {
  const txExec = await secretjs.tx.compute.executeContract(
    {
      sender: secretjs.address,
      contract: contractAddress,
      // codeHash,
      msg: {
        set_viewing_key: {
          key: viewingKey,
        },
      },
    },
    {
      gasLimit: 5000000,
    }
  );
  console.log(fromUtf8(txExec.data[0]))

  const status = JSON.parse(fromUtf8(txExec.data[0])).set_viewing_key.status;
  expect(status).toBe('success');
  return status;
}

async function performSomething(contractAddress: string, contractCodeHash: string, secretjs: SecretNetworkClient, senderAddress: string, operation: string, param: object) {
  let addItemMsg = new MsgExecuteContract({
      sender: senderAddress,
      contract: contractAddress,
      codeHash: contractCodeHash, // Test MsgExecuteContract without codeHash
      msg: { [operation]: {...param} },
      sentFunds: [],
    });

  const tx = await secretjs.tx.broadcast([addItemMsg], {
    gasLimit: 5000000,
  });

  expect(tx.code).toBe(0);
  expect(getValueFromRawLog(tx.rawLog, "message.action")).toBe("execute");
  expect(getValueFromRawLog(tx.rawLog, "wasm.contract_address")).toBe(
    contractAddress
  );
  // Check decryption
  // expect(tx.arrayLog![4].key).toBe("contract_address");
  // expect(tx.arrayLog![4].value).toBe(contractAddress);
}

function create_update_msg(quantity: number, userAddress: string) {
  let user_details: UserItemDetails =  {
      account_address: userAddress,
      contact_data: {
          delivery_address: "user delivery address",
          email: "user@email.com",
      },
      quantity: quantity,
  };
  let update_item_data: UpdateItemData = {
      category: "laptops",
      url: "www.item.com",
      user_details: user_details,
  };
  return update_item_data
}

async function sendFunds(secretjs: SecretNetworkClient, fromAddress: string, toAddress: string) {
  const tx = await secretjs.tx.bank.send(
    {
      fromAddress: fromAddress,
      toAddress: toAddress,
      amount: [{ denom: "uscrt", amount: "10000000" }],
    },
    {
      gasLimit: 20_000,
    },
  );
}

function assert_fetched_data_after_update(
  fetched_data: Result,
  expected_len: number,
  expected_quantity: number,
  expected_group_size: number,
) {
  expect(fetched_data.user_items.length).toBe(expected_len);
  expect(fetched_data.user_items[0].url).toBe("www.item.com");
  expect(fetched_data.user_items[0].quantity).toBe(expected_quantity);
  expect(
      fetched_data.contact_data!.email
  ).toBe("user@email.com");
  expect(fetched_data.items[0].static_data.price).toBe("1000");
  expect(
      fetched_data.items[0].current_group_size
  ).toBe(expected_group_size);
  expect(fetched_data.status).toBe("success");
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

    const timeout = Date.now() + 300_000;
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
  let viewingKey: string = "wefhjyr";
  let sellerAddress: string;
  let userAddress: string;

  beforeAll(async () => {
    const { secretjs } = accounts[0];
    const userSecretjs = accounts[1].secretjs;
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
          msg: "hey, initialized",
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

    await performSetViewingKey(userSecretjs, contractAddress, viewingKey);

    sellerAddress = accounts[0].address;
    userAddress = accounts[1].address;
  });

  test("add new item", async () => {
    const { secretjs } = accounts[0];
    const userSecretjs = accounts[1].secretjs;
    let staticItemData: StaticItemData = {
      name: "Cool item",
      category: "laptops",
      url: "www.item.com",
      img_url: "www.image-item.com",
      seller_address: sellerAddress,
      seller_email: "seller@email.com",
      price: "1000",
      wanted_price: "900",
      group_size_goal: 10,
    };

    await performSomething(contractAddress, contractCodeHash, secretjs, sellerAddress, "add_item", staticItemData);

    const result = (await userSecretjs.query.compute.queryContract({
      address: contractAddress,
      codeHash: contractCodeHash,
      query: { get_items: {category: "laptops", address: userAddress, key: viewingKey} },
    })) as Result;

    expect(result.items[0].static_data.price).toStrictEqual('1000');
    expect(result.items[0].current_group_size).toStrictEqual(0);
  });


  test("update new user for item, goal not reached", async () => {
    const { secretjs } = accounts[0];
    const userSecretjs = accounts[1].secretjs;

    let staticItemData: StaticItemData = {
      name: "Cool item",
      category: "laptops",
      url: "www.item.com",
      img_url: "www.image-item.com",
      seller_address: sellerAddress,
      seller_email: "seller@email.com",
      price: "1000",
      wanted_price: "900",
      group_size_goal: 10,
    };

    await performSomething(contractAddress, contractCodeHash, secretjs, sellerAddress, "add_item", staticItemData);

    let update_item_data = create_update_msg(1, userSecretjs.address);
    await performSomething(contractAddress, contractCodeHash, userSecretjs, userAddress, "update_item", update_item_data);


    const result = (await userSecretjs.query.compute.queryContract({
      address: contractAddress,
      codeHash: contractCodeHash,
      query: { get_items: {category: "laptops", address: userAddress, key: viewingKey} },
    })) as Result;

    assert_fetched_data_after_update(result, 1, 1, 1);
  
  });

  test("update new user for item, goal reached", async () => {
    const { secretjs } = accounts[0];
    const userSecretjs = accounts[1].secretjs;

    let staticItemData: StaticItemData = {
      name: "Cool item",
      category: "laptops",
      url: "www.item.com",
      img_url: "www.image-item.com",
      seller_address: sellerAddress,
      seller_email: "seller@email.com",
      price: "1000",
      wanted_price: "900",
      group_size_goal: 10,
    };

    await performSomething(contractAddress, contractCodeHash, secretjs, sellerAddress, "add_item", staticItemData);
    await sendFunds(userSecretjs, userAddress, contractAddress);
    let update_item_data = create_update_msg(10, userAddress);

    // Send fund to the contact - to simulate what should happen in the client side
    const balance = await secretjs.query.bank.balance({
      address: userAddress,
      denom: "uscrt",
    });
    console.log("User balance is: ", balance.balance!.amount);
    await performSomething(contractAddress, contractCodeHash, userSecretjs, userAddress, "update_item", update_item_data);


    const result = (await userSecretjs.query.compute.queryContract({
      address: contractAddress,
      codeHash: contractCodeHash,
      query: { get_items: {category: "laptops", address: userAddress, key: viewingKey} },
    })) as Result;

    expect(result.user_items.length).toBe(0);
    expect(result.contact_data).toBe(null);
    expect(result.items.length).toBe(0);
    expect(result.status).toBe("success");
//   Todo: Verify accounts balances (contract, seller)
  });
});
