// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Reusable Slider/Range component with Pierre design system styling
// ABOUTME: Features visual markers, gradient track, and value display

import React, { useState, useCallback } from 'react';

export interface SliderProps {
  label?: string;
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  showValue?: boolean;
  valueFormatter?: (value: number) => string;
  markers?: number[];
  markerLabels?: Record<number, string>;
  disabled?: boolean;
  helpText?: string;
  className?: string;
}

export const Slider: React.FC<SliderProps> = ({
  label,
  value,
  onChange,
  min = 0,
  max = 100,
  step = 1,
  showValue = true,
  valueFormatter = (v) => v.toString(),
  markers = [],
  markerLabels = {},
  disabled = false,
  helpText,
  className = '',
}) => {
  const [isDragging, setIsDragging] = useState(false);

  const percentage = ((value - min) / (max - min)) * 100;

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(Number(e.target.value));
    },
    [onChange]
  );

  const getMarkerPosition = (markerValue: number) => {
    return ((markerValue - min) / (max - min)) * 100;
  };

  return (
    <div className={`w-full ${className}`}>
      {/* Label and Value */}
      <div className="flex items-center justify-between mb-2">
        {label && (
          <label className="text-sm font-medium text-pierre-gray-700">{label}</label>
        )}
        {showValue && (
          <span className="text-sm font-semibold text-pierre-violet">
            {valueFormatter(value)}
          </span>
        )}
      </div>

      {/* Slider Track */}
      <div className="relative pt-1 pb-6">
        {/* Background track */}
        <div className="absolute top-1/2 left-0 right-0 h-2 -translate-y-1/2 bg-pierre-gray-200 rounded-full">
          {/* Filled track with gradient */}
          <div
            className="absolute top-0 left-0 h-full bg-gradient-pierre-horizontal rounded-full transition-all duration-150"
            style={{ width: `${percentage}%` }}
          />
        </div>

        {/* Markers */}
        {markers.map((marker) => {
          const position = getMarkerPosition(marker);
          const isActive = value >= marker;
          return (
            <div
              key={marker}
              className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2"
              style={{ left: `${position}%` }}
            >
              <div
                className={`w-3 h-3 rounded-full border-2 transition-colors ${
                  isActive
                    ? 'bg-pierre-violet border-pierre-violet'
                    : 'bg-white border-pierre-gray-300'
                }`}
              />
              {markerLabels[marker] && (
                <span className="absolute top-5 left-1/2 -translate-x-1/2 text-xs text-pierre-gray-500 whitespace-nowrap">
                  {markerLabels[marker]}
                </span>
              )}
            </div>
          );
        })}

        {/* Range Input */}
        <input
          type="range"
          value={value}
          onChange={handleChange}
          onMouseDown={() => setIsDragging(true)}
          onMouseUp={() => setIsDragging(false)}
          onTouchStart={() => setIsDragging(true)}
          onTouchEnd={() => setIsDragging(false)}
          min={min}
          max={max}
          step={step}
          disabled={disabled}
          className={`
            absolute top-1/2 left-0 w-full -translate-y-1/2 h-2
            appearance-none bg-transparent cursor-pointer
            disabled:cursor-not-allowed disabled:opacity-50
            [&::-webkit-slider-thumb]:appearance-none
            [&::-webkit-slider-thumb]:w-5
            [&::-webkit-slider-thumb]:h-5
            [&::-webkit-slider-thumb]:rounded-full
            [&::-webkit-slider-thumb]:bg-white
            [&::-webkit-slider-thumb]:border-2
            [&::-webkit-slider-thumb]:border-pierre-violet
            [&::-webkit-slider-thumb]:shadow-md
            [&::-webkit-slider-thumb]:cursor-pointer
            [&::-webkit-slider-thumb]:transition-transform
            [&::-webkit-slider-thumb]:hover:scale-110
            ${isDragging ? '[&::-webkit-slider-thumb]:scale-110' : ''}
            [&::-moz-range-thumb]:appearance-none
            [&::-moz-range-thumb]:w-5
            [&::-moz-range-thumb]:h-5
            [&::-moz-range-thumb]:rounded-full
            [&::-moz-range-thumb]:bg-white
            [&::-moz-range-thumb]:border-2
            [&::-moz-range-thumb]:border-pierre-violet
            [&::-moz-range-thumb]:shadow-md
            [&::-moz-range-thumb]:cursor-pointer
          `}
        />
      </div>

      {/* Help Text */}
      {helpText && (
        <p className="text-sm text-pierre-gray-500">{helpText}</p>
      )}
    </div>
  );
};
