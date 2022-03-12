use std::collections::btree_set::Difference;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmMsg, WasmQuery, QueryRequest, CosmosMsg, Order, Addr
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Cw20QueryMsg, Cw20CoinVerified};
use cw20::{TokenInfoResponse, Balance};
use cw_utils::{maybe_addr};
use cw_storage_plus::Bound;
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg, StakerListResponse, StakerInfo
};
use crate::state::{
    Config, CONFIG, STAKERS
};

// Version info, for migration info
const CONTRACT_NAME: &str = "gfotstaking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DAILY_FOT_AMOUNT:u128 = 100_000_000_000u128;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = msg
        .owner
        .map_or(Ok(info.sender), |o| deps.api.addr_validate(&o))?;

    let config = Config {
        owner: Some(owner),
        fot_token_address: msg.fot_token_address,
        bfot_token_address:msg.bfot_token_address,
        gfot_token_address: msg.gfot_token_address,
        fot_amount: Uint128::zero(),
        gfot_amount: Uint128::zero(),
        last_time: 0u64
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { new_owner } => execute_update_config(deps, info, new_owner),
        ExecuteMsg::Receive(msg) => try_receive(deps, env, info, msg),
        ExecuteMsg::WithdrawFot {} => try_withdraw_fot(deps, env, info),
        ExecuteMsg::WithdrawGFot {} => try_withdraw_gfot(deps, env, info),
        ExecuteMsg::ClaimReward {} => try_claim_reward(deps, env, info),
        ExecuteMsg::Unstake {} => try_unstake(deps, env, info),
    }
}

pub fn update_total_reward (
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {

    let mut cfg = CONFIG.load(deps.storage)?;
    let before_time = cfg.last_time;
    cfg.last_time = env.block.time.seconds();
    
    let delta = cfg.last_time % 86400u64 - before_time % 86400u64;
    if delta > 0 {
        //distributing FOT total amount
        let tot_fot_amount = Uint128::from(DAILY_FOT_AMOUNT).checked_mul(Uint128::from(delta)).unwrap();
        
        let all: StdResult<Vec<_>> = STAKERS
            .range(deps.storage, None, None, Order::Ascending)
            .collect();
        if !all.is_ok() {
            return Err(ContractError::VerificationFailed {})
        }
        let list = all.ok().unwrap();
        let mut tot_amount = Uint128::zero();
        for (_addr, (amount, _reward)) in list.clone() {
            tot_amount += amount;
        }

        for (addr, (amount, reward)) in list.clone() {
            let mut new_reward = tot_fot_amount.checked_mul(reward).unwrap().checked_div(tot_amount).unwrap();
            new_reward = reward.checked_add(new_reward).unwrap();
            STAKERS.save(deps.storage, &addr.clone(), &(amount, new_reward))?;
        }
    }
    Ok(Response::default())
}

pub fn try_receive(
    deps: DepsMut, 
    env: Env,
    info: MessageInfo, 
    wrapper: Cw20ReceiveMsg
) -> Result<Response, ContractError> {
    
    let mut cfg = CONFIG.load(deps.storage)?;
    let _msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let balance = Cw20CoinVerified {
        address: info.sender.clone(),
        amount: wrapper.amount,
    };

    let user_addr = &deps.api.addr_validate(&wrapper.sender)?;

    // Staking case
    if info.sender == cfg.gfot_token_address {

        let (mut amount, mut _reward) = STAKERS.load(deps.storage, &user_addr.clone())?;
        amount = amount + balance.amount;
        STAKERS.save(deps.storage, &user_addr.clone(), &(amount, _reward))?;
        
        cfg.gfot_amount = cfg.gfot_amount + balance.amount;
        CONFIG.save(deps.storage, &cfg)?;

        update_total_reward(deps, env)?;

        return Ok(Response::new()
            .add_attributes(vec![
                attr("action", "stake"),
                attr("address", user_addr),
                attr("amount", balance.amount)
            ]));

    } else if info.sender == cfg.fot_token_address {
        //Just receive in contract cache and update config
        cfg.fot_amount = cfg.fot_amount + balance.amount;
        CONFIG.save(deps.storage, &cfg)?;

        return Ok(Response::new()
            .add_attributes(vec![
                attr("action", "fund"),
                attr("address", user_addr),
                attr("amount", balance.amount),
            ]));

    } else {
        return Err(ContractError::UnacceptableToken {})
    }
}

pub fn try_claim_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo
) -> Result<Response, ContractError> {

    let (amount, reward) = STAKERS.load(deps.storage, &info.sender.clone())?;
    if reward == Uint128::zero() {
        return Err(ContractError::NoReward {});
    }
    let mut cfg = CONFIG.load(deps.storage)?;
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: cfg.fot_token_address.clone().into(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.clone().into(),
            amount: reward,
        })?,
        funds: vec![],
    };

    if cfg.fot_amount < reward {
        return Err(ContractError::NotEnoughFOT {});
    }
    cfg.fot_amount -= reward;
    CONFIG.save(deps.storage, &cfg)?;
    STAKERS.save(deps.storage, &info.sender.clone(), &(amount, Uint128::zero()))?;

    update_total_reward(deps, env)?;
    // return Ok(Response::new());
    return Ok(Response::new()
        .add_message(exec_cw20_transfer)
        .add_attributes(vec![
            attr("action", "claim_reward"),
            attr("address", info.sender.clone()),
            attr("fot_amount", reward),
        ]));
}

pub fn try_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo
) -> Result<Response, ContractError> {

    let (amount, reward) = STAKERS.load(deps.storage, &info.sender.clone())?;
    if amount == Uint128::zero() {
        return Err(ContractError::NoStaked {});
    }
    let mut cfg = CONFIG.load(deps.storage)?;
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: cfg.gfot_token_address.clone().into(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.clone().into(),
            amount,
        })?,
        funds: vec![],
    };
    if cfg.gfot_amount < amount {
        return Err(ContractError::NotEnoughgFOT {});
    }
    cfg.gfot_amount -= amount;
    CONFIG.save(deps.storage, &cfg)?;
    STAKERS.save(deps.storage, &info.sender.clone(), &(Uint128::zero(), reward))?;

    update_total_reward(deps, env)?;
    // return Ok(Response::new());
    return Ok(Response::new()
        .add_message(exec_cw20_transfer)
        .add_attributes(vec![
            attr("action", "claim_reward"),
            attr("address", info.sender.clone()),
            attr("fot_amount", reward),
        ]));
}

pub fn check_owner(
    deps: &DepsMut,
    info: &MessageInfo
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {})
    }
    Ok(Response::new().add_attribute("action", "check_owner"))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<String>,
) -> Result<Response, ContractError> {
    // authorize owner
    check_owner(&deps, &info)?;
    
    //test code for checking if check_owner works well
    // return Err(ContractError::InvalidInput {});
    // if owner some validated to addr, otherwise set to none
    let mut tmp_owner = None;
    if let Some(addr) = new_owner {
        tmp_owner = Some(deps.api.addr_validate(&addr)?)
    }

    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.owner = tmp_owner;
        Ok(exists)
    })?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn try_withdraw_fot(deps: DepsMut, env:Env, info: MessageInfo) -> Result<Response, ContractError> {

    check_owner(&deps, &info)?;
    let mut cfg = CONFIG.load(deps.storage)?;
    
    let fot_amount = cfg.fot_amount;
    cfg.fot_amount = Uint128::zero();
    CONFIG.save(deps.storage, &cfg)?;

    // create transfer cw20 msg
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: cfg.fot_token_address.clone().into(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.clone().into(),
            amount: fot_amount,
        })?,
        funds: vec![],
    };

    update_total_reward(deps, env)?;
    // return Ok(Response::new());
    return Ok(Response::new()
        .add_message(exec_cw20_transfer)
        .add_attributes(vec![
            attr("action", "fot_withdraw_all"),
            attr("address", info.sender.clone()),
            attr("fot_amount", fot_amount),
        ]));
}

pub fn try_withdraw_gfot(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {

    check_owner(&deps, &info)?;
    let mut cfg = CONFIG.load(deps.storage)?;
    
    let gfot_amount = cfg.gfot_amount;
    cfg.gfot_amount = Uint128::zero();
    CONFIG.save(deps.storage, &cfg)?;

    // create transfer cw20 msg
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: cfg.gfot_token_address.clone().into(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.clone().into(),
            amount: gfot_amount,
        })?,
        funds: vec![],
    };

    update_total_reward(deps, env)?;
    // return Ok(Response::new());
    return Ok(Response::new()
        .add_message(exec_cw20_transfer)
        .add_attributes(vec![
            attr("action", "gfot_withdraw_all"),
            attr("address", info.sender.clone()),
            attr("gfot_amount", gfot_amount),
        ]));
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} 
            => to_binary(&query_config(deps)?),
        QueryMsg::Staker {address} 
            => to_binary(&query_staker(deps, address)?),
        QueryMsg::ListStakers {start_after, limit} 
            => to_binary(&query_list_stakers(deps, start_after, limit)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner.map(|o| o.into()),
        fot_token_address: cfg.fot_token_address.into(),
        bfot_token_address: cfg.bfot_token_address.into(),
        gfot_token_address: cfg.gfot_token_address.into(),
        fot_amount: cfg.fot_amount,
        gfot_amount: cfg.gfot_amount
    })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn query_staker(deps: Deps, address: Addr) -> StdResult<StakerInfo> {
    
    let (amount, reward) = STAKERS.load(deps.storage, &address.clone())?;
        
    // let (amount, reward) = STAKERS.may_load(deps.storage, &address)?;
    let staker = StakerInfo {
        address,
        amount,
        reward
    };
    Ok(staker)
}
fn query_list_stakers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<StakerListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.map(|addr| Bound::exclusive(addr.as_ref()));

    let stakers = STAKERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.map(|(address, (amount, reward))| StakerInfo {
                address: address.into(),
                amount,
                reward
            })
        })
        .collect::<StdResult<_>>()?;

    Ok(StakerListResponse { stakers })
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}
