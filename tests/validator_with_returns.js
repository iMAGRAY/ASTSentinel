// Legitimate validator with hardcoded returns after logic
function isValidEmail(email) {
  // Check if email is provided
  if (!email || typeof email !== 'string') {
    return false;
  }
  
  // Check minimum length
  if (email.length < 3) {
    return false;
  }
  
  // Check for @ symbol
  if (!email.includes('@')) {
    return false;
  }
  
  // Check email format with regex
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!emailRegex.test(email)) {
    return false;
  }
  
  // Check for common typos
  const parts = email.split('@');
  if (parts[1] === 'gmial.com' || parts[1] === 'gmai.com') {
    return false; // Common Gmail typos
  }
  
  // All validations passed
  return true;
}

function validatePassword(password) {
  if (!password || password.length < 8) {
    return { valid: false, error: 'Password must be at least 8 characters' };
  }
  
  if (!/[A-Z]/.test(password)) {
    return { valid: false, error: 'Password must contain uppercase letter' };
  }
  
  if (!/[0-9]/.test(password)) {
    return { valid: false, error: 'Password must contain a number' };
  }
  
  return { valid: true };
}

module.exports = { isValidEmail, validatePassword };