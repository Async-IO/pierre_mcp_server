import { useWebSocket } from '../hooks/useWebSocket';

interface RealTimeIndicatorProps {
  className?: string;
}

export default function RealTimeIndicator({ className = '' }: RealTimeIndicatorProps) {
  const { isConnected, lastMessage } = useWebSocket();
  
  const getStatusColor = () => {
    if (!isConnected) return 'bg-red-500';
    return 'bg-green-500';
  };

  const getStatusText = () => {
    if (!isConnected) return 'Disconnected';
    return 'Live Updates';
  };

  const getLastUpdateInfo = () => {
    if (!lastMessage) return null;
    
    switch (lastMessage.type) {
      case 'usage_update':
        return `API key usage updated (${lastMessage.requests_today} today)`;
      case 'system_stats':
        return `System stats: ${lastMessage.total_requests_today} requests today`;
      case 'success':
        return lastMessage.message;
      case 'error':
        return `Error: ${lastMessage.message}`;
      default:
        return 'Real-time update received';
    }
  };

  return (
    <div className={`flex items-center space-x-2 ${className}`}>
      <div className="flex items-center space-x-2">
        <div className={`w-2 h-2 rounded-full ${getStatusColor()} ${isConnected ? 'animate-pulse' : ''}`}></div>
        <span className="text-sm text-gray-600">{getStatusText()}</span>
      </div>
      
      {lastMessage && (
        <div className="text-xs text-gray-500 max-w-md truncate">
          {getLastUpdateInfo()}
        </div>
      )}
    </div>
  );
}