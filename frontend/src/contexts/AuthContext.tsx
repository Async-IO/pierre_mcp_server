import { useState, useEffect } from 'react';
import { apiService } from '../services/api';
import { AuthContext } from './auth';
import type { User } from './auth';

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Check for stored user info on app start
    const storedUser = localStorage.getItem('user');
    const storedToken = localStorage.getItem('jwt_token');

    if (storedUser) {
      setUser(JSON.parse(storedUser));
    }
    if (storedToken) {
      setToken(storedToken);
    }

    setIsLoading(false);

    // Listen for auth failures from API service
    const handleAuthFailure = () => {
      logout();
    };

    window.addEventListener('auth-failure', handleAuthFailure);

    return () => {
      window.removeEventListener('auth-failure', handleAuthFailure);
    };
  }, []);

  const login = async (email: string, password: string) => {
    const response = await apiService.login(email, password);
    const { csrf_token, jwt_token, user: userData } = response;

    // Store CSRF token in API service
    apiService.setCsrfToken(csrf_token);

    // Store JWT token for WebSocket authentication
    if (jwt_token) {
      setToken(jwt_token);
      localStorage.setItem('jwt_token', jwt_token);
    }

    // Store user info in state and localStorage
    setUser(userData);
    localStorage.setItem('user', JSON.stringify(userData));
  };

  const logout = () => {
    setUser(null);
    setToken(null);

    // Clear user info and token from localStorage
    localStorage.removeItem('user');
    localStorage.removeItem('jwt_token');

    // Clear CSRF token from API service
    apiService.clearCsrfToken();
    apiService.clearUser();

    // Optionally call logout endpoint to clear cookies
    apiService.logout().catch((error) => {
      console.error('Logout API call failed:', error);
      // Continue with local cleanup even if API fails
    });
  };

  const value = {
    user,
    token,
    isAuthenticated: !!user,
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

