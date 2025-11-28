// ABOUTME: Reusable Skeleton loading components with Pierre design system styling
// ABOUTME: Features shimmer animation and variants for cards, text, charts

import React from 'react';

interface SkeletonBaseProps {
  className?: string;
  style?: React.CSSProperties;
}

// Base skeleton with shimmer animation
export const Skeleton: React.FC<SkeletonBaseProps> = ({ className = '', style }) => {
  return (
    <div
      className={`
        bg-gradient-to-r from-pierre-gray-200 via-pierre-gray-100 to-pierre-gray-200
        bg-[length:200%_100%] animate-shimmer rounded
        ${className}
      `}
      style={style}
    />
  );
};

// Text line skeleton
interface TextSkeletonProps extends SkeletonBaseProps {
  lines?: number;
  lastLineWidth?: string;
}

export const TextSkeleton: React.FC<TextSkeletonProps> = ({
  lines = 3,
  lastLineWidth = '60%',
  className = '',
}) => {
  return (
    <div className={`space-y-2 ${className}`}>
      {Array.from({ length: lines }).map((_, index) => (
        <Skeleton
          key={index}
          className="h-4"
          style={{ width: index === lines - 1 ? lastLineWidth : '100%' } as React.CSSProperties}
        />
      ))}
    </div>
  );
};

// Card skeleton
export const CardSkeleton: React.FC<SkeletonBaseProps> = ({ className = '' }) => {
  return (
    <div className={`bg-white rounded-lg border border-pierre-gray-200 p-6 ${className}`}>
      <div className="flex items-center gap-4 mb-4">
        <Skeleton className="w-12 h-12 rounded-full" />
        <div className="flex-1">
          <Skeleton className="h-4 w-32 mb-2" />
          <Skeleton className="h-3 w-24" />
        </div>
      </div>
      <TextSkeleton lines={3} />
    </div>
  );
};

// Stat card skeleton
export const StatCardSkeleton: React.FC<SkeletonBaseProps> = ({ className = '' }) => {
  return (
    <div className={`bg-white rounded-lg border border-pierre-gray-200 p-6 ${className}`}>
      <Skeleton className="h-8 w-20 mb-2" />
      <Skeleton className="h-4 w-24" />
    </div>
  );
};

// Table row skeleton
interface TableRowSkeletonProps extends SkeletonBaseProps {
  columns?: number;
}

export const TableRowSkeleton: React.FC<TableRowSkeletonProps> = ({ columns = 4, className = '' }) => {
  return (
    <div className={`flex items-center gap-4 p-4 border-b border-pierre-gray-200 ${className}`}>
      {Array.from({ length: columns }).map((_, index) => (
        <Skeleton key={index} className="h-4 flex-1" />
      ))}
    </div>
  );
};

// Table skeleton
interface TableSkeletonProps extends SkeletonBaseProps {
  rows?: number;
  columns?: number;
}

export const TableSkeleton: React.FC<TableSkeletonProps> = ({ rows = 5, columns = 4, className = '' }) => {
  return (
    <div className={`bg-white rounded-lg border border-pierre-gray-200 overflow-hidden ${className}`}>
      {/* Header */}
      <div className="flex items-center gap-4 p-4 bg-pierre-gray-50 border-b border-pierre-gray-200">
        {Array.from({ length: columns }).map((_, index) => (
          <Skeleton key={index} className="h-4 flex-1" />
        ))}
      </div>
      {/* Rows */}
      {Array.from({ length: rows }).map((_, index) => (
        <TableRowSkeleton key={index} columns={columns} />
      ))}
    </div>
  );
};

// Chart skeleton
export const ChartSkeleton: React.FC<SkeletonBaseProps> = ({ className = '' }) => {
  return (
    <div className={`bg-white rounded-lg border border-pierre-gray-200 p-6 ${className}`}>
      <div className="flex items-end gap-2 h-48">
        {[40, 65, 45, 80, 55, 70, 50, 85, 60, 75, 45, 90].map((height, index) => (
          <Skeleton
            key={index}
            className="flex-1 rounded-t"
            style={{ height: `${height}%` } as React.CSSProperties}
          />
        ))}
      </div>
      <div className="flex justify-between mt-4">
        {Array.from({ length: 6 }).map((_, index) => (
          <Skeleton key={index} className="h-3 w-8" />
        ))}
      </div>
    </div>
  );
};

// Avatar skeleton
interface AvatarSkeletonProps extends SkeletonBaseProps {
  size?: 'sm' | 'md' | 'lg';
}

export const AvatarSkeleton: React.FC<AvatarSkeletonProps> = ({ size = 'md', className = '' }) => {
  const sizeClasses = {
    sm: 'w-8 h-8',
    md: 'w-10 h-10',
    lg: 'w-12 h-12',
  };

  return <Skeleton className={`${sizeClasses[size]} rounded-full ${className}`} />;
};

// List skeleton
interface ListSkeletonProps extends SkeletonBaseProps {
  items?: number;
  showAvatar?: boolean;
}

export const ListSkeleton: React.FC<ListSkeletonProps> = ({
  items = 5,
  showAvatar = true,
  className = '',
}) => {
  return (
    <div className={`space-y-4 ${className}`}>
      {Array.from({ length: items }).map((_, index) => (
        <div key={index} className="flex items-center gap-3 p-3 bg-pierre-gray-50 rounded-lg">
          {showAvatar && <AvatarSkeleton size="md" />}
          <div className="flex-1">
            <Skeleton className="h-4 w-32 mb-2" />
            <Skeleton className="h-3 w-48" />
          </div>
          <Skeleton className="h-6 w-16 rounded-full" />
        </div>
      ))}
    </div>
  );
};

// Zone editor skeleton for fitness configuration
export const ZoneEditorSkeleton: React.FC<SkeletonBaseProps> = ({ className = '' }) => {
  return (
    <div className={`bg-white rounded-lg border border-pierre-gray-200 p-6 ${className}`}>
      <Skeleton className="h-6 w-48 mb-4" />
      <div className="space-y-3">
        {Array.from({ length: 5 }).map((_, index) => (
          <div key={index} className="flex items-center gap-4">
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-8 flex-1 rounded-full" />
            <Skeleton className="h-4 w-16" />
          </div>
        ))}
      </div>
    </div>
  );
};
