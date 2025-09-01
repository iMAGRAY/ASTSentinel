// Edge case 10: Simple middleware functions
function cors(req, res, next) {
  res.header('Access-Control-Allow-Origin', '*');
  res.header('Access-Control-Allow-Methods', 'GET,POST,PUT,DELETE');
  res.header('Access-Control-Allow-Headers', 'Content-Type,Authorization');
  next();
}

function authenticate(req, res, next) {
  const token = req.headers.authorization;
  
  if (!token) {
    return res.status(401).json({ error: 'No token provided' });
  }
  
  jwt.verify(token, process.env.JWT_SECRET, (err, decoded) => {
    if (err) {
      return res.status(401).json({ error: 'Invalid token' });
    }
    req.user = decoded;
    next();
  });
}

function rateLimiter(limit = 100) {
  const requests = new Map();
  
  return (req, res, next) => {
    const ip = req.ip;
    const now = Date.now();
    const windowMs = 60000; // 1 minute
    
    if (!requests.has(ip)) {
      requests.set(ip, { count: 1, resetTime: now + windowMs });
      return next();
    }
    
    const record = requests.get(ip);
    if (now > record.resetTime) {
      record.count = 1;
      record.resetTime = now + windowMs;
      return next();
    }
    
    if (record.count >= limit) {
      return res.status(429).json({ error: 'Rate limit exceeded' });
    }
    
    record.count++;
    next();
  };
}