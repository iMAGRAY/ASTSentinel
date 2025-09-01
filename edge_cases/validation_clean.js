// Edge case 8: Clean validation without placeholders
class ValidationService {
  constructor(userRepository) {
    this.userRepo = userRepository;
  }
  
  isValidEmail(email) {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
  }
  
  hasPermission(user, action) {
    if (!user || !user.roles) return false;
    return user.roles.includes(action);
  }
  
  parseNumber(str) {
    const num = parseInt(str, 10);
    return isNaN(num) ? null : num;
  }
  
  async getUserRole(userId) {
    if (!userId) return null;
    
    const user = await this.userRepo.findById(userId);
    if (!user) return null;
    if (user.isAdmin) return 'admin';
    if (user.isModerator) return 'moderator';
    return 'user';
  }
  
  validatePassword(password) {
    if (!password || password.length < 8) return false;
    if (!/[A-Z]/.test(password)) return false;
    if (!/[a-z]/.test(password)) return false;
    if (!/[0-9]/.test(password)) return false;
    return true;
  }
}