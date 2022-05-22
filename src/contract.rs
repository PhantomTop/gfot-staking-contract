use std::collections::btree_set::Difference;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
    WasmMsg, WasmQuery, QueryRequest, CosmosMsg, Order, Addr, Decimal, Storage, Api
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Cw20QueryMsg, Cw20CoinVerified};
use cw20::{TokenInfoResponse, Balance};
use cw_utils::{maybe_addr};
use cw_storage_plus::Bound;
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, CollectionInfo, CollectionListResponse
};
use crate::state::{
    Config, CONFIG, COLLECTIONS
};

// Version info, for migration info
const CONTRACT_NAME: &str = "nft-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = info.sender;

    let config = Config {
        owner,
        count: 0u32
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
        ExecuteMsg::AddCollection {collection_addr, cw721_addr} => execute_add_collection(deps, info, collection_addr, cw721_addr),
        ExecuteMsg::RemoveCollection {id} => execute_remove_collection(deps, info, id),
        ExecuteMsg::RemoveAllCollection {  } => execute_remove_all_collection(deps, info)
    }
}

pub fn check_owner(
    deps: &DepsMut,
    info: &MessageInfo
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    
    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {})
    }
    Ok(Response::new().add_attribute("action", "check_owner"))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Addr,
) -> Result<Response, ContractError> {
    // authorize owner
    check_owner(&deps, &info)?;
    
    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.owner = new_owner;
        Ok(exists)
    })?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_add_collection(
    deps: DepsMut,
    info: MessageInfo,
    collection_addr: Addr,
    cw721_addr: Addr
) -> Result<Response, ContractError> {

    let mut cfg = CONFIG.load(deps.storage)?;
    cfg.count += 1;
    CONFIG.save(deps.storage, &cfg);

    COLLECTIONS.save(deps.storage, cfg.count, &(collection_addr, cw721_addr))?;
    Ok(Response::new()
        .add_attribute("action", "add_collection")
        .add_attribute("collection_addr", collection_addr)
        .add_attribute("cw721_addr", cw721_addr)
    )
}

pub fn execute_remove_collection(
    deps: DepsMut,
    info: MessageInfo,
    id: u32
) -> Result<Response, ContractError>{
    check_owner(&deps, &info)?;
    COLLECTIONS.remove(deps.storage, id);
    Ok(Response::new()
        .add_attribute("action", "remove_collection")
       
    )
}

pub fn execute_remove_all_collection(
    deps: DepsMut,
    info: MessageInfo
) -> Result<Response, ContractError> {
    // authorize owner
    check_owner(&deps, &info)?;

    let collections:StdResult<Vec<_>> = COLLECTIONS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| map_collection(item))
        .collect();

    if collections.is_err() {
        return Err(ContractError::Map2ListFailed {})
    }
    
    for item in collections.unwrap() {
        COLLECTIONS.remove(deps.storage, item.id);
    }
    
    Ok(Response::new().add_attribute("action", "remove_all_collection"))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} 
            => to_binary(&query_config(deps)?),
        QueryMsg::Collection {id} 
            => to_binary(&query_collection(deps, id)?),
        QueryMsg::ListCollections {} 
            => to_binary(&query_list_collections(deps)?)
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner,
        count: cfg.count
    })
}

pub fn query_collection(deps: Deps, id: u32) -> StdResult<CollectionInfo> {
    let exists = COLLECTIONS.may_load(deps.storage, id)?;
    let (mut collection_addr, mut cw721_addr);
    if exists.is_some() {
        (collection_addr, cw721_addr) = exists.unwrap();
    } 
    Ok(CollectionInfo {
        id,
        collection_addr,
        cw721_addr
    })
}

pub fn query_list_collections(deps: Deps) 
-> StdResult<CollectionListResponse> {
    let collections:StdResult<Vec<_>> = COLLECTIONS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| map_collection(item))
        .collect();

    Ok(CollectionListResponse {
        list: collections?
    })
}

fn map_collection(
    item: StdResult<(u32, (Addr, Addr))>,
) -> StdResult<CollectionInfo> {
    item.map(|(id, (collection_addr, cw721_addr))| {
        CollectionInfo {
            id,
            collection_addr,
            cw721_addr
        }
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

