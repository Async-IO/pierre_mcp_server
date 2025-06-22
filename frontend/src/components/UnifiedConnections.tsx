import { useState } from 'react';
import { Button } from './ui';
import ApiKeyList from './ApiKeyList';
import CreateApiKey from './CreateApiKey';
import A2AClientList from './A2AClientList';
import CreateA2AClient from './CreateA2AClient';
import AdminTokenList from './AdminTokenList';
import CreateAdminToken from './CreateAdminToken';
import AdminTokenDetails from './AdminTokenDetails';
import type { AdminToken, CreateAdminTokenResponse } from '../types/api';

type ConnectionType = 'api-keys' | 'oauth-apps' | 'admin-tokens';
type View = 'overview' | 'create' | 'details';

interface TokenSuccessModalProps {
  isOpen: boolean;
  onClose: () => void;
  response: CreateAdminTokenResponse;
}

const TokenSuccessModal: React.FC<TokenSuccessModalProps> = ({ isOpen, onClose, response }) => {
  const [copied, setCopied] = useState(false);

  const copyToClipboard = async () => {
    try {
      await navigator.clipboard.writeText(response.jwt_token);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy token:', err);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl max-w-2xl mx-4 w-full p-6">
        <div className="mb-6">
          <h3 className="text-lg font-semibold text-pierre-gray-900">
            🎉 Admin Token Generated Successfully
          </h3>
          <p className="text-pierre-gray-600 mt-1">
            Your new admin token is ready for use
          </p>
        </div>
        
        <div className="space-y-6">
          <div className="bg-pierre-yellow-50 border border-pierre-yellow-200 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <svg className="w-6 h-6 text-pierre-yellow-600 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.5 0L4.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
              <div>
                <h4 className="font-medium text-pierre-yellow-800">Important Security Notice</h4>
                <p className="text-sm text-pierre-yellow-700 mt-1">
                  This is the only time the full token will be displayed. Please copy it now and store it securely.
                </p>
              </div>
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-pierre-gray-700 mb-2">
              JWT Token
            </label>
            <div className="relative">
              <textarea
                className="w-full px-4 py-3 border border-pierre-gray-300 rounded-md font-mono text-xs resize-none"
                value={response.jwt_token}
                readOnly
                rows={8}
                onClick={(e) => e.currentTarget.select()}
              />
              <Button
                variant="secondary"
                size="sm"
                className="absolute top-2 right-2"
                onClick={copyToClipboard}
              >
                {copied ? '✓ Copied!' : 'Copy'}
              </Button>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-pierre-gray-500">Service:</span>
              <span className="ml-2 font-medium">{response.admin_token.service_name}</span>
            </div>
            <div>
              <span className="text-pierre-gray-500">Prefix:</span>
              <span className="ml-2 font-mono">{response.admin_token.token_prefix}...</span>
            </div>
          </div>

          <div className="flex gap-3 pt-4 border-t border-pierre-gray-200">
            <Button onClick={onClose} className="flex-1">
              I've Saved the Token Securely
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default function UnifiedConnections() {
  const [activeConnectionType, setActiveConnectionType] = useState<ConnectionType>('admin-tokens');
  const [activeView, setActiveView] = useState<View>('overview');
  const [selectedToken, setSelectedToken] = useState<AdminToken | null>(null);
  const [showTokenSuccess, setShowTokenSuccess] = useState(false);
  const [tokenResponse, setTokenResponse] = useState<CreateAdminTokenResponse | null>(null);

  const renderTabs = () => (
    <div className="border-b border-pierre-gray-200 mb-8">
      <nav className="-mb-px flex space-x-8">
        <button
          className={`tab ${activeConnectionType === 'admin-tokens' ? 'tab-active' : ''}`}
          onClick={() => {
            setActiveConnectionType('admin-tokens');
            setActiveView('overview');
            setSelectedToken(null);
          }}
        >
          <span>🛡️</span>
          <span>Admin Tokens</span>
        </button>
        <button
          className={`tab ${activeConnectionType === 'api-keys' ? 'tab-active' : ''}`}
          onClick={() => {
            setActiveConnectionType('api-keys');
            setActiveView('overview');
            setSelectedToken(null);
          }}
        >
          <span>🔑</span>
          <span>API Keys</span>
        </button>
        <button
          className={`tab ${activeConnectionType === 'oauth-apps' ? 'tab-active' : ''}`}
          onClick={() => {
            setActiveConnectionType('oauth-apps');
            setActiveView('overview');
            setSelectedToken(null);
          }}
        >
          <span>🤖</span>
          <span>Connected Apps</span>
        </button>
      </nav>
    </div>
  );

  const handleTokenCreated = (response: CreateAdminTokenResponse) => {
    setTokenResponse(response);
    setShowTokenSuccess(true);
    setActiveView('overview');
  };

  const handleTokenSuccess = () => {
    setShowTokenSuccess(false);
    setTokenResponse(null);
  };

  const renderContent = () => {
    // Details view for admin tokens
    if (activeView === 'details' && selectedToken) {
      return (
        <AdminTokenDetails
          token={selectedToken}
          onBack={() => {
            setActiveView('overview');
            setSelectedToken(null);
          }}
          onTokenUpdated={() => {
            // Refresh will happen automatically via react-query
          }}
        />
      );
    }

    // Create views
    if (activeView === 'create') {
      if (activeConnectionType === 'admin-tokens') {
        return (
          <CreateAdminToken
            onBack={() => setActiveView('overview')}
            onTokenCreated={handleTokenCreated}
          />
        );
      } else if (activeConnectionType === 'api-keys') {
        return (
          <div>
            <div className="mb-6">
              <Button
                variant="secondary"
                onClick={() => setActiveView('overview')}
                size="sm"
              >
                ← Back to API Keys
              </Button>
            </div>
            <CreateApiKey />
          </div>
        );
      } else {
        return (
          <div>
            <div className="mb-6">
              <Button
                variant="secondary"
                onClick={() => setActiveView('overview')}
                size="sm"
              >
                ← Back to Connected Apps
              </Button>
            </div>
            <CreateA2AClient
              onSuccess={() => setActiveView('overview')}
              onCancel={() => setActiveView('overview')}
            />
          </div>
        );
      }
    }

    // Overview content
    if (activeConnectionType === 'admin-tokens') {
      return (
        <AdminTokenList
          onCreateToken={() => setActiveView('create')}
          onViewDetails={() => {
            // Find the token by ID from the data we should have
            // TODO: Implement token details lookup by ID
            setActiveView('details');
          }}
        />
      );
    } else if (activeConnectionType === 'api-keys') {
      return (
        <div>
          <div className="flex justify-between items-center mb-6">
            <div>
              <h2 className="text-2xl font-bold text-pierre-gray-900">API Keys</h2>
              <p className="text-pierre-gray-600 mt-1">
                Manage your API keys for programmatic access to Pierre Fitness API
              </p>
            </div>
            <Button
              onClick={() => setActiveView('create')}
              className="flex items-center space-x-2"
            >
              <span>+</span>
              <span>Create API Key</span>
            </Button>
          </div>
          <ApiKeyList />
        </div>
      );
    }

    // OAuth Apps (A2A) content
    return (
      <div>
        <div className="flex justify-between items-center mb-6">
          <div>
            <h2 className="text-2xl font-bold text-pierre-gray-900">Connected Apps</h2>
            <p className="text-pierre-gray-600 mt-1">
              Manage OAuth apps for secure agent-to-agent communication
            </p>
          </div>
          <Button
            onClick={() => setActiveView('create')}
            className="flex items-center space-x-2"
          >
            <span>+</span>
            <span>Register App</span>
          </Button>
        </div>
        <A2AClientList />
      </div>
    );
  };

  return (
    <div className="space-y-0">
      {tokenResponse && (
        <TokenSuccessModal
          isOpen={showTokenSuccess}
          onClose={handleTokenSuccess}
          response={tokenResponse}
        />
      )}
      {renderTabs()}
      {renderContent()}
    </div>
  );
}