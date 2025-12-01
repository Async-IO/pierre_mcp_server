// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

import { useWebSocketContext } from '../hooks/useWebSocketContext';

interface RealTimeIndicatorProps {
  className?: string;
}

export default function RealTimeIndicator({ className = '' }: RealTimeIndicatorProps) {
  const { isConnected } = useWebSocketContext();

  const getStatusColor = () => {
    if (!isConnected) return 'bg-red-500';
    return 'bg-green-500';
  };

  const getStatusText = () => {
    if (!isConnected) return 'Disconnected';
    return 'Live Updates';
  };

  return (
    <div className={`flex items-center space-x-1 ${className}`}>
      <div className={`w-1.5 h-1.5 rounded-full ${getStatusColor()} ${isConnected ? 'animate-pulse' : ''}`}></div>
      <span className="text-gray-500">{getStatusText()}</span>
    </div>
  );
}