import { useState, useEffect } from 'react';
import { apiService } from '../services/api';
import { AuthContext } from './auth';
import type { User } from './auth';

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Check for stored token on app start
    const storedToken = localStorage.getItem('auth_token');
    const storedUser = localStorage.getItem('user');
    
    if (storedToken && storedUser) {
      setToken(storedToken);
      setUser(JSON.parse(storedUser));
      apiService.setAuthToken(storedToken);
      
      // Check if token needs refresh
      apiService.checkAndRefreshToken().catch((error) => {
        console.error('Token refresh failed on startup:', error);
        // Don't logout on startup refresh failure, let it handle naturally
      });
    }
    
    setIsLoading(false);

    // Listen for auth failures from API service
    const handleAuthFailure = () => {
      logout();
    };

    // Listen for token updates from API service
    const handleTokenUpdate = (event: Event) => {
      const customEvent = event as CustomEvent<{ token: string }>;
      const newToken = customEvent.detail.token;
      setToken(newToken);
    };

    window.addEventListener('auth-failure', handleAuthFailure);
    window.addEventListener('token-updated', handleTokenUpdate);
    
    // Set up periodic token refresh check (every 10 minutes)
    const refreshInterval = setInterval(() => {
      if (apiService.getToken()) {
        apiService.checkAndRefreshToken().catch((error) => {
          console.error('Periodic token refresh failed:', error);
        });
      }
    }, 10 * 60 * 1000); // 10 minutes

    return () => {
      window.removeEventListener('auth-failure', handleAuthFailure);
      window.removeEventListener('token-updated', handleTokenUpdate);
      clearInterval(refreshInterval);
    };
  }, []);

  const login = async (email: string, password: string) => {
    const response = await apiService.login(email, password);
    const { jwt_token, user: userData } = response;
    
    setToken(jwt_token);
    setUser(userData);
    
    // Store in localStorage
    localStorage.setItem('auth_token', jwt_token);
    localStorage.setItem('user', JSON.stringify(userData));
    
    // Set token in API service
    apiService.setAuthToken(jwt_token);
  };

  const logout = () => {
    setToken(null);
    setUser(null);
    
    // Clear localStorage
    localStorage.removeItem('auth_token');
    localStorage.removeItem('user');
    
    // Clear token from API service
    apiService.setAuthToken(null);
  };

  const value = {
    user,
    token,
    isAuthenticated: !!token,
    isLoading,
    loading: isLoading, // For test compatibility
    login,
    logout,
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
}

