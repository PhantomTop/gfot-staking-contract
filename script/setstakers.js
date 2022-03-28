#!/usr/bin/env node
/*jshint esversion: 8 */


/* eslint-disable @typescript-eslint/naming-convention */
import { toBase64, toUtf8 } from '@cosmjs/encoding';
import axios from 'axios';
import * as fs from "fs";
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { GasPrice } from '@cosmjs/stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';

const config = {
    endpoint: 'https://rpc-juno.itastakers.com:443',
    bech32prefix: 'juno',
    feeDenom: 'ujuno',
    gasPrice: GasPrice.fromString('0.0025ujuno'),
    mnemonic: 'ignore extra below blame trip hero pipe enlist gentle absent cricket dice woman choice neither rifle model bubble cabbage puppy ski dance pink own',
};

async function setup() {
    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(config.mnemonic, {
        prefix: config.bech32prefix,
    });
    const { address } = (await wallet.getAccounts())[0];
    const options = {
        prefix: config.bech32prefix,
        gasPrice: config.gasPrice,
    };
    const client = await SigningCosmWasmClient.connectWithSigner(
        config.endpoint,
        wallet,
        options
    );

    // now ensure there is a balance
    console.log(`Querying balance of ${address}`);
    const {denom, amount} = await client.getBalance(address, config.feeDenom);
    console.log(`Got ${amount} ${denom}`);
    if (!amount || amount === "0") {
        console.log('Please add tokens to your account before uploading')
    }
  
    return { address, client };
}

const oldContractAddr = "juno1kh65msgczpzlvat9x94n82v8qnlmtkmjees4pjc9wppckw07d32se6qp6t";
const newContractAddr = "juno1kh65msgczpzlvat9x94n82v8qnlmtkmjees4pjc9wppckw07d32se6qp6t";

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function main() {
    const { address, client } = await setup();

    console.log('execute addstakers')
    var oldstakers = []
    var newstakers = []
    var queryMsg = {
        limit: 30
    };

    let zerocnt = 0;
    while(true) {
        
        let arr = await client.queryContractSmart(
            oldContractAddr,
            {
                list_stakers: queryMsg

            }
        );
        
        let list = arr.stakers;

        if (list.length == 0)
            break;

        list.forEach(element => {
            // console.log(element)
            oldstakers.push(element)
        });

        queryMsg.start_after = list[list.length-1].address;
        console.log(oldstakers.length)
        // if (oldstakers.length < 1530)
            await sleep(300)
        // else 
        // {
        //     await sleep(3000)
        //     await client.execute(
        //         address,
        //         newContractAddr,
        //         { 
        //             add_stakers: {
        //                 stakers: list
        //             }
        //         },
        //         'auto',
        //         '',
        //         []
        //     );
        // }
        
        
    }

    // for ( let staker in stakers) {
        // console.log(JSON.stringify(staker))

    // }
    console.log(oldstakers.length)
    fs.writeFileSync(`oldstakerlist.json`, JSON.stringify(oldstakers))
    

}

main().then(
    () => {
      console.info('All done');
      process.exit(0);
    },
    (error) => {
      console.error(error);
      process.exit(1);
    }
  );