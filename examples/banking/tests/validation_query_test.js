import http from 'k6/http';
import { check, sleep } from 'k6';
import exec from 'k6/execution';

const serverUrl = 'http://localhost:8080';
const USERS = 200;
const CONFIG=  {headers: { 'Content-Type': 'application/json' }};

export let options = {
    vus: USERS,
    duration: '10s',
};

function buildAmountPayload(amount){
    return JSON.stringify({
        amount: amount
    });
}

function requestOpenAccount(id, amount) {
    return http.post(`${serverUrl}/account/${id}/open`, buildAmountPayload(amount), CONFIG);
}


function requestDeposit(id, amount) {
    return http.post(`${serverUrl}/account/${id}/deposit`, buildAmountPayload(amount), CONFIG);
}

function requestWithdraw(id, amount) {
    return http.post(`${serverUrl}/account/${id}/withdraw`, buildAmountPayload(amount), CONFIG);
}

function requestTransfer(id, beneficiary_id, amount) {
    return http.post(`${serverUrl}/account/${id}/transfer/${beneficiary_id}`, buildAmountPayload(amount), CONFIG);
}

export function setup() {
   // Register accounts 
   for(let i = 1; i <= USERS; i ++){
   	requestOpenAccount(`account_${i}`, 100);
   }
}

export default function (data) {
    let id = exec.vu.idInTest;
    let account = `account_${id}`;

    let res;
    if(id === 1){
    	res = requestWithdraw(account, 1);
    }else{
    	res = requestTransfer(account, 'account_1' , 20);
    }

    check(res, {
    	'OK': (r) => res.status === 200,
    	'ERR Bad': (r) => res.status === 400,
    	'ERR Conflict': (r) => res.status === 500,
    });
}
