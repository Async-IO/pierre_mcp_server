import { useState } from 'react';
import { Button } from './ui';
import ApiKeyList from './ApiKeyList';
import CreateApiKey from './CreateApiKey';
import A2AClientList from './A2AClientList';
import CreateA2AClient from './CreateA2AClient';

type ConnectionType = 'api-keys' | 'oauth-apps';
type View = 'overview' | 'create';

export default function UnifiedConnections() {
  const [activeConnectionType, setActiveConnectionType] = useState<ConnectionType>('api-keys');
  const [activeView, setActiveView] = useState<View>('overview');

  const renderTabs = () => (
    <div className="border-b border-gray-200 mb-8">
      <nav className="-mb-px flex space-x-8">
        <button
          className={`
            py-2 px-1 border-b-2 font-medium text-sm
            ${activeConnectionType === 'api-keys' 
              ? 'border-api-blue text-api-blue' 
              : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}
          `}
          onClick={() => {
            setActiveConnectionType('api-keys');
            setActiveView('overview');
          }}
        >
          <div className="flex items-center space-x-2">
            <span>🔑</span>
            <span>API Keys</span>
          </div>
        </button>
        <button
          className={`
            py-2 px-1 border-b-2 font-medium text-sm
            ${activeConnectionType === 'oauth-apps' 
              ? 'border-api-blue text-api-blue' 
              : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}
          `}
          onClick={() => {
            setActiveConnectionType('oauth-apps');
            setActiveView('overview');
          }}
        >
          <div className="flex items-center space-x-2">
            <span>🤖</span>
            <span>Connected Apps</span>
          </div>
        </button>
      </nav>
    </div>
  );

  const renderContent = () => {
    if (activeView === 'create') {
      if (activeConnectionType === 'api-keys') {
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
    if (activeConnectionType === 'api-keys') {
      return (
        <div>
          <div className="flex justify-between items-center mb-6">
            <div>
              <h2 className="text-2xl font-bold text-gray-900">API Keys</h2>
              <p className="text-gray-600 mt-1">
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
            <h2 className="text-2xl font-bold text-gray-900">Connected Apps</h2>
            <p className="text-gray-600 mt-1">
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
      {renderTabs()}
      {renderContent()}
    </div>
  );
}