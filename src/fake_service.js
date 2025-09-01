// Fake implementation for testing
function getUserData() {
  // TODO: implement real API call
  return { id: 1, name: "Test User" };
}

function calculatePrice() {
  // Just return hardcoded value for now
  return 99.99;
}

module.exports = { getUserData, calculatePrice };