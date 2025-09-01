// JavaScript file with obvious security issues
const API_KEY = "sk-live-secret123";
const password = "admin123";

function getUserData(userId) {
    // SQL injection
    const query = `SELECT * FROM users WHERE id = ${userId}`;
    return query;
}

function executeCmd(cmd) {
    // Command injection
    require('child_process').exec(`echo ${cmd}`);
}

console.log("Debug test file");