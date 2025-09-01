// Edge case 2: Legitimate use of randomness
class SecurityUtils {
  // Generate secure session ID
  generateSessionId() {
    const timestamp = Date.now().toString(36);
    const randomPart = Math.random().toString(36).substring(2);
    const randomBytes = crypto.randomBytes(16).toString('hex');
    
    return `${timestamp}_${randomPart}_${randomBytes}`;
  }
  
  // Generate CSRF token  
  generateCSRFToken() {
    return crypto.randomUUID();
  }
  
  // Add salt to passwords
  generateSalt(length = 16) {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let salt = '';
    
    for (let i = 0; i < length; i++) {
      salt += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    
    return salt;
  }
}