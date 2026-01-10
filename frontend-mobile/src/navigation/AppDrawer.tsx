// ABOUTME: Main app drawer navigation for authenticated users
// ABOUTME: Contains Chat, Connections, and Settings screens with custom drawer content

import React from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  ScrollView,
  SafeAreaView,
  Image,
} from 'react-native';
import {
  createDrawerNavigator,
  DrawerContentScrollView,
  type DrawerContentComponentProps,
} from '@react-navigation/drawer';
import { ChatScreen } from '../screens/chat/ChatScreen';
import { ConnectionsScreen } from '../screens/connections/ConnectionsScreen';
import { SettingsScreen } from '../screens/settings/SettingsScreen';
import { useAuth } from '../contexts/AuthContext';
import { colors, spacing, fontSize, borderRadius } from '../constants/theme';

export type AppDrawerParamList = {
  Chat: undefined;
  Connections: undefined;
  Settings: undefined;
};

const Drawer = createDrawerNavigator<AppDrawerParamList>();

function CustomDrawerContent(props: DrawerContentComponentProps) {
  const { user } = useAuth();
  const { navigation, state } = props;

  const menuItems = [
    { name: 'Chat', icon: '💬', label: 'Chat' },
    { name: 'Connections', icon: '🔗', label: 'Connections' },
    { name: 'Settings', icon: '⚙️', label: 'Settings' },
  ];

  return (
    <SafeAreaView style={styles.drawerContainer}>
      {/* Header */}
      <View style={styles.drawerHeader}>
        <Image
          source={require('../../assets/pierre-logo.png')}
          style={styles.drawerLogo}
          resizeMode="contain"
        />
        <Text style={styles.appName}>Pierre</Text>
      </View>

      {/* User Info */}
      <View style={styles.userSection}>
        <View style={styles.userAvatar}>
          <Text style={styles.userAvatarText}>
            {user?.display_name?.[0]?.toUpperCase() || user?.email?.[0]?.toUpperCase() || 'U'}
          </Text>
        </View>
        <View style={styles.userInfo}>
          <Text style={styles.userName} numberOfLines={1}>
            {user?.display_name || 'User'}
          </Text>
          <Text style={styles.userEmail} numberOfLines={1}>
            {user?.email}
          </Text>
        </View>
      </View>

      {/* Menu Items */}
      <ScrollView style={styles.menuContainer}>
        {menuItems.map((item, index) => {
          const isActive = state.routeNames[state.index] === item.name;

          return (
            <TouchableOpacity
              key={item.name}
              style={[styles.menuItem, isActive && styles.menuItemActive]}
              onPress={() => navigation.navigate(item.name)}
            >
              <Text style={[styles.menuIcon, isActive && styles.menuTextActive]}>
                {item.icon}
              </Text>
              <Text style={[styles.menuLabel, isActive && styles.menuTextActive]}>
                {item.label}
              </Text>
            </TouchableOpacity>
          );
        })}
      </ScrollView>

      {/* Footer */}
      <View style={styles.drawerFooter}>
        <Text style={styles.footerText}>Pierre v1.0.0</Text>
      </View>
    </SafeAreaView>
  );
}

export function AppDrawer() {
  return (
    <Drawer.Navigator
      drawerContent={(props) => <CustomDrawerContent {...props} />}
      screenOptions={{
        headerShown: false,
        drawerStyle: {
          backgroundColor: colors.background.secondary,
          width: 280,
        },
        drawerType: 'front',
        overlayColor: 'rgba(0, 0, 0, 0.5)',
      }}
    >
      <Drawer.Screen name="Chat" component={ChatScreen} />
      <Drawer.Screen name="Connections" component={ConnectionsScreen} />
      <Drawer.Screen name="Settings" component={SettingsScreen} />
    </Drawer.Navigator>
  );
}

const styles = StyleSheet.create({
  drawerContainer: {
    flex: 1,
    backgroundColor: colors.background.secondary,
  },
  drawerHeader: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingTop: spacing.lg,
    paddingBottom: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  drawerLogo: {
    width: 40,
    height: 40,
    marginRight: spacing.sm,
  },
  appName: {
    fontSize: fontSize.xl,
    fontWeight: '700',
    color: colors.text.primary,
  },
  userSection: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border.subtle,
  },
  userAvatar: {
    width: 44,
    height: 44,
    borderRadius: 22,
    backgroundColor: colors.primary[700],
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: spacing.sm,
  },
  userAvatarText: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.text.primary,
  },
  userInfo: {
    flex: 1,
  },
  userName: {
    fontSize: fontSize.md,
    fontWeight: '600',
    color: colors.text.primary,
  },
  userEmail: {
    fontSize: fontSize.sm,
    color: colors.text.secondary,
  },
  menuContainer: {
    flex: 1,
    paddingTop: spacing.md,
  },
  menuItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    marginHorizontal: spacing.sm,
    borderRadius: borderRadius.md,
  },
  menuItemActive: {
    backgroundColor: colors.primary[600] + '20',
  },
  menuIcon: {
    fontSize: 20,
    marginRight: spacing.md,
    color: colors.text.secondary,
  },
  menuLabel: {
    fontSize: fontSize.md,
    color: colors.text.secondary,
  },
  menuTextActive: {
    color: colors.primary[500],
  },
  drawerFooter: {
    paddingHorizontal: spacing.lg,
    paddingVertical: spacing.md,
    borderTopWidth: 1,
    borderTopColor: colors.border.subtle,
  },
  footerText: {
    fontSize: fontSize.xs,
    color: colors.text.tertiary,
    textAlign: 'center',
  },
});
