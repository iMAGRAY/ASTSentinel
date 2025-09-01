// Edge case 6: Configuration with environment-specific values
const config = {
  development: {
    database: {
      host: process.env.DB_HOST || 'localhost',
      port: process.env.DB_PORT || 5432,
      name: process.env.DB_NAME || 'app_dev'
    },
    cache: {
      enabled: false, // disable cache in dev for testing
      ttl: 0
    }
  },
  
  production: {
    database: {
      host: process.env.DB_HOST,
      port: process.env.DB_PORT,
      name: process.env.DB_NAME
    },
    cache: {
      enabled: true,
      ttl: 3600
    }
  },
  
  test: {
    database: {
      host: 'localhost',
      port: 5432,
      name: 'app_test'
    },
    cache: {
      enabled: false
    }
  }
};

const currentEnv = process.env.NODE_ENV || 'development';
module.exports = config[currentEnv];