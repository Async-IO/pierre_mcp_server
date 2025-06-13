import { createContext } from 'react';

interface User {
  user_id: string;
  email: string;
  display_name?: string;
}

interface AuthContextType {
  user: User | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  loading: boolean; // For test compatibility
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
}

export const AuthContext = createContext<AuthContextType | undefined>(undefined);
export type { User, AuthContextType };