#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmMsg, WasmQuery, QueryRequest, CosmosMsg
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Cw20QueryMsg};
use cw20::{TokenInfoResponse};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, 
};
use crate::state::{
    Config, CONFIG
};

// Version info, for migration info
const CONTRACT_NAME: &str = "bfotburn";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const BFOT_START_AMOUNT:u128 = 100_000_000_000_000u128;
const STEP_AMOUNT:u128 = 10_000_000_000u128;
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
        bfot_token_address: deps.api.addr_validate(&msg.bfot_token_address)?,
        gfot_token_address: deps.api.addr_validate(&msg.gfot_token_address)?,
        bfot_burn_amount: Uint128::zero(),
        gfot_sent_amount: Uint128::zero(),
        gfot_current_amount: Uint128::zero()
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { new_owner } => execute_update_config(deps, info, new_owner),
        ExecuteMsg::Receive(msg) => try_receive(deps, info, msg),
        ExecuteMsg::WithdrawAll {} => try_withdraw_all(deps, info),
    }
}



pub fn try_receive(
    deps: DepsMut, 
    info: MessageInfo, 
    wrapper: Cw20ReceiveMsg
) -> Result<Response, ContractError> {
    
    let mut cfg = CONFIG.load(deps.storage)?;
    // let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    // let balance = Balance::Cw20(Cw20CoinVerified {
    //     address: info.sender,
    //     amount: wrapper.amount,
    // });

    // match msg {
    //     ReceiveMsg::Fot {} => {
    //         execute_fot(deps, balance, &deps.api.addr_validate(&wrapper.sender)?);
    //     },
    //     ReceiveMsg::Bfot {} => {
    //         execute_bfot(deps, balance);
    //     }
    // }

    // let fot_token_info: TokenInfoResponse =
    //     deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
    //         contract_addr: cfg.bfot_token_address.clone().into(),
    //         msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    //     }))?;

    // let gfot_left_amount = Uint128::from(fot_token_info.total_supply);
    let user_addr = &deps.api.addr_validate(&wrapper.sender)?;

    if info.sender == cfg.bfot_token_address {

        if cfg.gfot_current_amount < Uint128::from(STEP_AMOUNT) {
            return Err(ContractError::NotEnoughgFOT {})
        }
        let bfot_received_amount = wrapper.amount;
        let bfot_accept_amount = Uint128::from(BFOT_START_AMOUNT) + cfg.gfot_sent_amount;
        
        if bfot_received_amount < bfot_accept_amount {
            return Err(ContractError::NotEnoughbFOT {bfot_accept_amount});
        }

        let bfot_return_amount = bfot_received_amount.checked_sub(bfot_accept_amount).unwrap();
        
        cfg.bfot_burn_amount = cfg.bfot_burn_amount + bfot_accept_amount;
        cfg.gfot_sent_amount = cfg.gfot_sent_amount + Uint128::from(STEP_AMOUNT);
        cfg.gfot_current_amount = cfg.gfot_current_amount - Uint128::from(STEP_AMOUNT);

        CONFIG.save(deps.storage, &cfg)?;
        
        let mut messages:Vec<CosmosMsg> = vec![];
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.gfot_token_address.into(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: user_addr.clone().into(),
                amount: Uint128::from(STEP_AMOUNT),
            })?,
        }));
        if bfot_return_amount > Uint128::from(0u128) {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cfg.bfot_token_address.clone().into(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: user_addr.into(),
                    amount: bfot_return_amount,
                })?,
            }));
        }
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.bfot_token_address.clone().into(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: bfot_accept_amount,
            })?,
        }));
        
        return Ok(Response::new()
            .add_messages(messages)
            .add_attributes(vec![
                attr("action", "send_gfot_burn_bfot"),
                attr("address", user_addr),
                attr("bfot_burn_amount", bfot_accept_amount),
                attr("bfot_return_amount", bfot_return_amount),
                attr("gfot_amount", Uint128::from(STEP_AMOUNT)),
            ]));

    } else if info.sender == cfg.gfot_token_address {
        //Just receive in contract cache and update config
        cfg.gfot_current_amount = cfg.gfot_current_amount + wrapper.amount;
        CONFIG.save(deps.storage, &cfg)?;

        return Ok(Response::new()
            .add_attributes(vec![
                attr("action", "receive_gfot"),
                attr("address", user_addr),
                attr("gfot_amount", wrapper.amount),
            ]));

    } else {
        return Err(ContractError::UnacceptableToken {})
    }
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


pub fn try_withdraw_all(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {

    check_owner(&deps, &info)?;
    let mut cfg = CONFIG.load(deps.storage)?;
    
    let gfot_current_amount = cfg.gfot_current_amount;
    let gfot_token_address = cfg.gfot_token_address.clone();
    cfg.gfot_current_amount = Uint128::zero();

    CONFIG.save(deps.storage, &cfg)?;

    // create transfer cw20 msg
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: gfot_token_address.into(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.clone().into(),
            amount: gfot_current_amount,
        })?,
        funds: vec![],
    };

    // return Ok(Response::new());
    return Ok(Response::new()
        .add_message(exec_cw20_transfer)
        .add_attributes(vec![
            attr("action", "gfot_withdraw_all"),
            attr("address", info.sender.clone()),
            attr("gfot_amount", gfot_current_amount),
        ]));
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner.map(|o| o.into()),
        bfot_token_address: cfg.bfot_token_address.into(),
        gfot_token_address: cfg.gfot_token_address.into(),
        bfot_burn_amount: cfg.bfot_burn_amount,
        gfot_sent_amount: cfg.gfot_sent_amount,
        gfot_current_amount: cfg.gfot_current_amount,
        bfot_expected_amount: cfg.gfot_sent_amount + Uint128::from(BFOT_START_AMOUNT)
    })
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
