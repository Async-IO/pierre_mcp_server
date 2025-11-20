import { useState, useEffect } from 'react';
import { apiService } from '../services/api';
import { AuthContext } from './auth';
import type { User } from './auth';

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Check for stored user info on app start
    const storedUser = localStorage.getItem('user');

    if (storedUser) {
      setUser(JSON.parse(storedUser));
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
    const { csrf_token, user: userData } = response;

    // Store CSRF token in API service
    apiService.setCsrfToken(csrf_token);

    // Store user info in state and localStorage
    setUser(userData);
    localStorage.setItem('user', JSON.stringify(userData));
  };

  const logout = () => {
    setUser(null);

    // Clear user info from localStorage
    localStorage.removeItem('user');

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
    token: null, // Deprecated - kept for backward compatibility
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

