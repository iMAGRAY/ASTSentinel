// Edge case: minimal logic but legitimate
function isProduction() {
  return process.env.NODE_ENV === 'production';
}

function isEmpty(obj) {
  return Object.keys(obj).length === 0;
}

function getDefaultConfig() {
  // Returns default configuration
  return {
    timeout: 5000,
    retries: 3,
    debug: false
  };
}

function formatCurrency(amount) {
  if (typeof amount !== 'number') {
    return '$0.00';
  }
  return '$' + amount.toFixed(2);
}

module.exports = { isProduction, isEmpty, getDefaultConfig, formatCurrency };