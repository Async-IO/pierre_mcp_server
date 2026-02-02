// ABOUTME: Authentication context provider for Pierre Mobile app
// ABOUTME: Manages user auth state, login/logout, and persists tokens with AsyncStorage

import React, { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import { authApi, onAuthFailure } from '../services/api';
import { signOutFromFirebase } from '../firebase';
import type { User, FirebaseLoginResponse } from '../types';

interface AuthContextType {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (email: string, password: string) => Promise<void>;
  loginWithFirebase: (idToken: string) => Promise<FirebaseLoginResponse>;
  logout: () => Promise<void>;
  register: (email: string, password: string, displayName?: string) => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Initialize auth state from storage
  useEffect(() => {
    const initAuth = async () => {
      try {
        const hasToken = await authApi.initializeAuth();
        if (hasToken) {
          const storedUser = await authApi.getStoredUser();
          if (storedUser) {
            setUser(storedUser);
          }
        }
      } catch (error) {
        console.error('Failed to initialize auth:', error);
      } finally {
        setIsLoading(false);
      }
    };

    initAuth();
  }, []);

  // Listen for auth failures (401 responses)
  useEffect(() => {
    const unsubscribe = onAuthFailure(() => {
      setUser(null);
    });
    return unsubscribe;
  }, []);

  const login = useCallback(async (email: string, password: string) => {
    const response = await authApi.login({ email, password });

    // OAuth2 response contains access_token and user info
    const loginUser: User = response.user || {
      user_id: '',
      email,
      is_admin: false,
      role: 'user',
      user_status: 'active',
    };

    await authApi.storeAuth(response.access_token, response.csrf_token || '', loginUser);
    setUser(loginUser);
  }, []);

  const loginWithFirebase = useCallback(async (idToken: string): Promise<FirebaseLoginResponse> => {
    const response = await authApi.loginWithFirebase({ idToken });

    // Store auth tokens and user info
    await authApi.storeAuth(response.jwt_token, response.csrf_token, response.user);
    setUser(response.user);

    return response;
  }, []);

  const logout = useCallback(async () => {
    await authApi.logout();
    await signOutFromFirebase();
    setUser(null);
  }, []);

  const register = useCallback(async (email: string, password: string, displayName?: string) => {
    await authApi.register({ email, password, display_name: displayName });
    // After registration, user needs to log in (or wait for approval if pending)
  }, []);

  const value: AuthContextType = {
    user,
    isAuthenticated: !!user && user.user_status === 'active',
    isLoading,
    login,
    loginWithFirebase,
    logout,
    register,
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextType {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}

export { AuthContext };
