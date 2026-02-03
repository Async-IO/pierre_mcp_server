// ABOUTME: Jest setup file for Pierre Mobile app tests
// ABOUTME: Configures mocks for React Native modules

// Suppress known React Testing Library act() warnings for async state updates
// These occur when components have async useEffect/useFocusEffect that update state
const originalError = console.error;
console.error = (...args) => {
  const message = args[0];
  if (
    typeof message === 'string' &&
    message.includes('inside a test was not wrapped in act')
  ) {
    return;
  }
  originalError.apply(console, args);
};

// Mock react-native-gesture-handler
jest.mock('react-native-gesture-handler', () => {
  const View = require('react-native').View;
  const TouchableOpacity = require('react-native').TouchableOpacity;
  return {
    GestureHandlerRootView: View,
    TouchableOpacity: TouchableOpacity,
    Swipeable: View,
    DrawerLayout: View,
    State: {},
    PanGestureHandler: View,
    BaseButton: View,
    RectButton: View,
    ScrollView: require('react-native').ScrollView,
    FlatList: require('react-native').FlatList,
  };
});

// Mock AsyncStorage
jest.mock('@react-native-async-storage/async-storage', () =>
  require('@react-native-async-storage/async-storage/jest/async-storage-mock')
);

// Mock expo-secure-store
jest.mock('expo-secure-store', () => ({
  getItemAsync: jest.fn(() => Promise.resolve(null)),
  setItemAsync: jest.fn(() => Promise.resolve()),
  deleteItemAsync: jest.fn(() => Promise.resolve()),
}));

// Mock expo-web-browser
jest.mock('expo-web-browser', () => ({
  openBrowserAsync: jest.fn(() => Promise.resolve({ type: 'success' })),
}));

// Mock expo-haptics
jest.mock('expo-haptics', () => ({
  impactAsync: jest.fn(() => Promise.resolve()),
  notificationAsync: jest.fn(() => Promise.resolve()),
  selectionAsync: jest.fn(() => Promise.resolve()),
  ImpactFeedbackStyle: {
    Light: 'light',
    Medium: 'medium',
    Heavy: 'heavy',
  },
  NotificationFeedbackType: {
    Success: 'success',
    Warning: 'warning',
    Error: 'error',
  },
}));

// Mock expo-speech-recognition
const mockEventListeners = {};
jest.mock('expo-speech-recognition', () => ({
  ExpoSpeechRecognitionModule: {
    isRecognitionAvailable: jest.fn(() => true),
    start: jest.fn(),
    stop: jest.fn(),
    abort: jest.fn(),
    requestPermissionsAsync: jest.fn(() => Promise.resolve({ granted: true })),
    getPermissionsAsync: jest.fn(() => Promise.resolve({ granted: true })),
    addListener: jest.fn((eventName, listener) => {
      mockEventListeners[eventName] = listener;
      return { remove: jest.fn() };
    }),
  },
  useSpeechRecognitionEvent: jest.fn((eventName, listener) => {
    mockEventListeners[eventName] = listener;
  }),
  // Export for tests to trigger events
  __mockEventListeners: mockEventListeners,
  __triggerEvent: (eventName, data) => {
    if (mockEventListeners[eventName]) {
      mockEventListeners[eventName](data);
    }
  },
  __clearMockListeners: () => {
    Object.keys(mockEventListeners).forEach(key => delete mockEventListeners[key]);
  },
}));

// Mock react-native-toast-message
jest.mock('react-native-toast-message', () => {
  const View = require('react-native').View;
  return {
    __esModule: true,
    default: View,
    show: jest.fn(),
    hide: jest.fn(),
  };
});

// Mock react-native-safe-area-context
jest.mock('react-native-safe-area-context', () => {
  const React = require('react');
  const View = require('react-native').View;
  return {
    SafeAreaProvider: ({ children }) => React.createElement(View, { testID: 'safe-area-provider' }, children),
    SafeAreaView: ({ children, ...props }) => React.createElement(View, props, children),
    useSafeAreaInsets: () => ({ top: 44, bottom: 34, left: 0, right: 0 }),
    useSafeAreaFrame: () => ({ x: 0, y: 0, width: 390, height: 844 }),
  };
});

// Mock expo-linear-gradient
jest.mock('expo-linear-gradient', () => {
  const React = require('react');
  const View = require('react-native').View;
  return {
    LinearGradient: ({ children, colors, testID, ...props }) =>
      React.createElement(View, { testID: testID || 'linear-gradient', ...props }, children),
  };
});

// Mock @shopify/flash-list - use FlatList as a drop-in replacement for tests
jest.mock('@shopify/flash-list', () => {
  const React = require('react');
  const FlatList = require('react-native').FlatList;
  return {
    FlashList: FlatList,
    FlashListRef: {},
  };
});

