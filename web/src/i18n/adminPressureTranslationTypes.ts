export interface AdminPressureTranslations {
  title: string
  description: string
  loading: string
  retry: string
  errorTitle: string
  emptyTitle: string
  emptyDescription: string
  userFallback: string
  summary: {
    currentPressure: string
    currentPressureHint: string
    currentPeak: string
    yesterdayDelta: string
    yesterdayDeltaHint: string
    activeUsers: string
    activeUsersHint: string
    distribution: string
    distributionHint: string
  }
  charts: {
    last24h: {
      title: string
      description: string
      currentLabel: string
      previousLabel: string
      averageLabel: string
    }
    userDistribution: {
      title: string
      description: string
      seriesLabel: string
      xAxisLabel: string
      yAxisLabel: string
      userCountLabel: string
      empty: string
    }
    last7d: {
      title: string
      description: string
      seriesLabel: string
      sma6hLabel: string
      sma24hLabel: string
    }
  }
}
