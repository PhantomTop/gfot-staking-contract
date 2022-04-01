#!/bin/bash

#Build Flag
PARAM=$1
####################################    Constants    ##################################################

#depends on mainnet or testnet
NODE="--node https://rpc-juno.itastakers.com:443"
CHAIN_ID=juno-1
DENOM="ujuno"

# origin addresses
# FOT_ADDRESS="juno1xmpenz0ykxfy8rxr3yc3d4dtqq4dpas4zz3xl6sh873us3vajlpshzp69d"
# BFOT_ADDRESS="juno1vaeuky9hqacenay9nmuualugvv54tdhyt2wsvhnjasx9s946hhmqaq3kh7"
# GFOT_ADDRESS="juno10ynpq4wchr4ruu6mhrfh29495ep4cja5vjnkhz3j5lrgcsap9vtssyeekl"

#GFOT is lp token, BFOT is not necessary, FOT is same

#SFOT-UST 
# GFOT_ADDRESS="juno1te6t7zar4jrme4re7za0vzxf72rjkwwzxrksu83505l89gdzcy9sd93v4c"
# FOT_ADDRESS="juno1xmpenz0ykxfy8rxr3yc3d4dtqq4dpas4zz3xl6sh873us3vajlpshzp69d"
# BFOT_ADDRESS="juno1vaeuky9hqacenay9nmuualugvv54tdhyt2wsvhnjasx9s946hhmqaq3kh7"

# #SFOT-BFOT 
GFOT_ADDRESS="juno19qetspgghczk5hvw3su602vjqqdhgl062eftgh897cdka6lny5sq6yhmg4"
FOT_ADDRESS="juno1xmpenz0ykxfy8rxr3yc3d4dtqq4dpas4zz3xl6sh873us3vajlpshzp69d"
BFOT_ADDRESS="juno1vaeuky9hqacenay9nmuualugvv54tdhyt2wsvhnjasx9s946hhmqaq3kh7"


##########################################################################################
#not depends
NODECHAIN=" $NODE --chain-id $CHAIN_ID"
TXFLAG=" $NODECHAIN --gas-prices 0.0025$DENOM --gas auto --gas-adjustment 1.3"
WALLET="--from fortis"

WASMFILE="artifacts/gfotstaking.wasm"

FILE_UPLOADHASH="uploadtx.txt"
FILE_CONTRACT_ADDR="contractaddr.txt"
FILE_CODE_ID="code.txt"

ADDR_FORTIS="juno1mp7wa6sxcstk2kwvt5czkz3eel8rcd06j93pq5"
###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################

#Instantiate Contract
Instantiate() {
    echo "================================================="
    echo "Instantiate Contract"
    
    #read from FILE_CODE_ID
    CODE_ID=$(cat $FILE_CODE_ID)
    junod tx wasm instantiate $CODE_ID '{"owner":"'$ADDR_FORTIS'", "fot_token_address":"'$FOT_ADDRESS'","bfot_token_address":"'$BFOT_ADDRESS'", "gfot_token_address":"'$GFOT_ADDRESS'", "daily_fot_amount":"100000000000000", "apy_prefix":"109500000", "reward_interval":86400, "lock_days":14}' --label "SFOT-UST LP Staking" $WALLET $TXFLAG -y
}

#Get Instantiated Contract Address
GetContractAddress() {
    echo "================================================="
    echo "Get contract address by code"
    
    #read from FILE_CODE_ID
    CODE_ID=$(cat $FILE_CODE_ID)
    junod query wasm list-contract-by-code $CODE_ID $NODECHAIN --output json
    CONTRACT_ADDR=$(junod query wasm list-contract-by-code $CODE_ID $NODECHAIN --output json | jq -r '.contracts[-1]')
    
    echo "Contract Address : "$CONTRACT_ADDR

    #save to FILE_CONTRACT_ADDR
    echo $CONTRACT_ADDR > $FILE_CONTRACT_ADDR
}


###################################################################################################
###################################################################################################
###################################################################################################
###################################################################################################
#Send initial tokens
SendFot() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $FOT_ADDRESS '{"send":{"amount":"18250000000000000","contract":"'$CONTRACT_GFOTSTAKING'","msg":""}}' $WALLET $TXFLAG -y
}

SendGFot() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $GFOT_ADDRESS '{"send":{"amount":"3249612324013","contract":"'$CONTRACT_GFOTSTAKING'","msg":""}}' $WALLET $TXFLAG -y
}

RemoveStaker() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"remove_staker":{"address":"'$ADDR_FORTIS'"}}' $WALLET $TXFLAG -y
}

RemoveAllStakers() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"remove_all_stakers":{}}' $WALLET $TXFLAG -y
}

WithdrawFot() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"withdraw_fot":{}}' $WALLET $TXFLAG -y
}

WithdrawGFot() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"withdraw_g_fot":{}}' $WALLET $TXFLAG -y
}

ClaimReward() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"claim_reward":{}}' $WALLET $TXFLAG -y
}

PrintUnstaking() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_GFOTSTAKING '{"unstaking":{"address":"'$ADDR_FORTIS'"}}' $NODECHAIN
}

Unstake() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"create_unstake":{"unstake_amount":"10000"}}' $WALLET $TXFLAG -y
}

FetchUnstake() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"fetch_unstake":{"index":0}}' $WALLET $TXFLAG -y
}

UpdateConfig() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"update_config":{"new_owner":"'$ADDR_FORTIS'"}}' $WALLET $TXFLAG -y
}

UpdateConstants() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod tx wasm execute $CONTRACT_GFOTSTAKING '{"update_constants":{"daily_fot_amount":"100000000000000", "apy_prefix":"109500000", "reward_interval": 86400, "lock_days":14, "enabled":true}}' $WALLET $TXFLAG -y
}

PrintConfig() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_GFOTSTAKING '{"config":{}}' $NODECHAIN
}

PrintStaker() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_GFOTSTAKING '{"staker":{"address":"'$ADDR_FORTIS'"}}' $NODECHAIN
}

PrintUnstaking() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_GFOTSTAKING '{"unstaking":{"address":"'$ADDR_FORTIS'"}}' $NODECHAIN
}

PrintListStakers() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_GFOTSTAKING '{"list_stakers":{}}' $NODECHAIN
}

PrintAPY() {
    CONTRACT_GFOTSTAKING=$(cat $FILE_CONTRACT_ADDR)
    junod query wasm contract-state smart $CONTRACT_GFOTSTAKING '{"apy":{}}' $NODECHAIN
}

#################################################################################
PrintWalletBalance() {
    echo "native balance"
    echo "========================================="
    junod query bank balances $ADDR_FORTIS $NODECHAIN
    echo "========================================="
    echo "FOT balance"
    echo "========================================="
    junod query wasm contract-state smart $FOT_ADDRESS '{"balance":{"address":"'$ADDR_FORTIS'"}}' $NODECHAIN
    echo "========================================="
    echo "BFOT balance"
    echo "========================================="
    junod query wasm contract-state smart $BFOT_ADDRESS '{"balance":{"address":"'$ADDR_FORTIS'"}}' $NODECHAIN
    echo "========================================="
    echo "GFOT balance"
    echo "========================================="
    junod query wasm contract-state smart $GFOT_ADDRESS '{"balance":{"address":"'$ADDR_FORTIS'"}}' $NODECHAIN
}

#################################### End of Function ###################################################
if [[ $PARAM == "" ]]; then
    Instantiate
sleep 10
    GetContractAddress
sleep 10
    SendFot
sleep 7
    SendGFot
sleep 10
    RemoveAllStakers
# sleep 5
#     Withdraw
sleep 7
    PrintConfig
sleep 7
    PrintWalletBalance
# sleep 7
#     RemoveStaker
# sleep 5
#     PrintStaker
sleep 5
    PrintListStakers
else
    $PARAM
fi

# OptimizeBuild
# Upload
# GetCode
# Instantiate
# GetContractAddress
# CreateEscrow
# TopUp

