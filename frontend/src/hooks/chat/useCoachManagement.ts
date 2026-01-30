// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// ABOUTME: Custom hook for managing coaches - CRUD operations, favorites, visibility
// ABOUTME: Extracted from ChatTab to improve separation of concerns

import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { coachesApi } from '../../services/api';
import { QUERY_KEYS } from '../../constants/queryKeys';

interface Coach {
  id: string;
  title: string;
  description: string | null;
  system_prompt: string;
  category: string;
  tags: string[];
  token_count: number;
  is_favorite: boolean;
  use_count: number;
  last_used_at: string | null;
  is_system: boolean;
  is_assigned: boolean;
}

interface CoachFormData {
  title: string;
  description: string;
  system_prompt: string;
  category: string;
}

const DEFAULT_FORM_DATA: CoachFormData = {
  title: '',
  description: '',
  system_prompt: '',
  category: 'Training',
};

export function useCoachManagement(enabled: boolean = true) {
  const queryClient = useQueryClient();

  // UI state
  const [showCoachModal, setShowCoachModal] = useState(false);
  const [showMyCoachesPanel, setShowMyCoachesPanel] = useState(false);
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [showHiddenCoaches, setShowHiddenCoaches] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [editingCoachId, setEditingCoachId] = useState<string | null>(null);
  const [coachFormData, setCoachFormData] = useState<CoachFormData>(DEFAULT_FORM_DATA);
  const [deleteConfirmation, setDeleteConfirmation] = useState<{ id: string; title: string } | null>(null);

  // Fetch coaches
  const { data: coachesData, isLoading: coachesLoading } = useQuery({
    queryKey: QUERY_KEYS.coaches.list(),
    queryFn: () => coachesApi.list(),
    staleTime: 5 * 60 * 1000,
    enabled: enabled && showMyCoachesPanel,
  });

  // Fetch hidden coaches
  const { data: hiddenCoachesData } = useQuery({
    queryKey: QUERY_KEYS.coaches.hidden(),
    queryFn: () => coachesApi.getHidden(),
    staleTime: 5 * 60 * 1000,
    enabled: enabled && showMyCoachesPanel,
  });

  // Filtered coaches based on search and category
  const filteredCoaches = useMemo(() => {
    let coaches = coachesData?.coaches ?? [];

    if (categoryFilter) {
      coaches = coaches.filter(c => c.category === categoryFilter);
    }

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      coaches = coaches.filter(c =>
        c.title.toLowerCase().includes(query) ||
        c.description?.toLowerCase().includes(query) ||
        c.tags.some(t => t.toLowerCase().includes(query))
      );
    }

    return coaches;
  }, [coachesData?.coaches, categoryFilter, searchQuery]);

  // Create coach mutation
  const createCoach = useMutation({
    mutationFn: (data: CoachFormData) => coachesApi.create(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.list() });
      setShowCoachModal(false);
      setCoachFormData(DEFAULT_FORM_DATA);
    },
  });

  // Update coach mutation
  const updateCoach = useMutation({
    mutationFn: ({ id, data }: { id: string; data: CoachFormData }) =>
      coachesApi.update(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.list() });
      setShowCoachModal(false);
      setEditingCoachId(null);
      setCoachFormData(DEFAULT_FORM_DATA);
    },
  });

  // Delete coach mutation
  const deleteCoach = useMutation({
    mutationFn: (id: string) => coachesApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.list() });
      setDeleteConfirmation(null);
    },
  });

  // Hide coach mutation
  const hideCoach = useMutation({
    mutationFn: (coachId: string) => coachesApi.hide(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.list() });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.hidden() });
    },
  });

  // Show coach mutation
  const showCoach = useMutation({
    mutationFn: (coachId: string) => coachesApi.show(coachId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.list() });
      queryClient.invalidateQueries({ queryKey: QUERY_KEYS.coaches.hidden() });
    },
  });

  // Handlers
  const handleOpenCoachModal = (coach?: Coach) => {
    if (coach) {
      setEditingCoachId(coach.id);
      setCoachFormData({
        title: coach.title,
        description: coach.description || '',
        system_prompt: coach.system_prompt,
        category: coach.category,
      });
    } else {
      setEditingCoachId(null);
      setCoachFormData(DEFAULT_FORM_DATA);
    }
    setShowCoachModal(true);
  };

  const handleCloseCoachModal = () => {
    setShowCoachModal(false);
    setEditingCoachId(null);
    setCoachFormData(DEFAULT_FORM_DATA);
  };

  const handleSaveCoach = () => {
    if (editingCoachId) {
      updateCoach.mutate({ id: editingCoachId, data: coachFormData });
    } else {
      createCoach.mutate(coachFormData);
    }
  };

  const handleDeleteCoach = (coach: { id: string; title: string }) => {
    setDeleteConfirmation({ id: coach.id, title: coach.title });
  };

  const handleConfirmDelete = () => {
    if (deleteConfirmation) {
      deleteCoach.mutate(deleteConfirmation.id);
    }
  };

  const handleCancelDelete = () => {
    setDeleteConfirmation(null);
  };

  const handleToggleVisibility = (coach: Coach) => {
    // If coach is hidden, show it; otherwise hide it
    const isHidden = hiddenCoachesData?.coaches?.some(c => c.id === coach.id);
    if (isHidden) {
      showCoach.mutate(coach.id);
    } else {
      hideCoach.mutate(coach.id);
    }
  };

  return {
    // UI state
    showCoachModal,
    showMyCoachesPanel,
    categoryFilter,
    showHiddenCoaches,
    searchQuery,
    editingCoachId,
    coachFormData,
    deleteConfirmation,

    // Setters
    setShowCoachModal,
    setShowMyCoachesPanel,
    setCategoryFilter,
    setShowHiddenCoaches,
    setSearchQuery,
    setCoachFormData,

    // Query data
    coaches: coachesData?.coaches ?? [],
    filteredCoaches,
    hiddenCoaches: hiddenCoachesData?.coaches ?? [],
    coachesLoading,

    // Mutations
    createCoach,
    updateCoach,
    deleteCoach,
    hideCoach,
    showCoach,

    // Handlers
    handleOpenCoachModal,
    handleCloseCoachModal,
    handleSaveCoach,
    handleDeleteCoach,
    handleConfirmDelete,
    handleCancelDelete,
    handleToggleVisibility,
  };
}

export type { Coach, CoachFormData };
