import { useMemo } from 'react'

import {
  CategoryScale,
  Chart as ChartJS,
  Filler,
  Legend,
  LinearScale,
  LineElement,
  PointElement,
  Tooltip,
  type ChartData,
  type ChartOptions,
  type ScriptableContext,
} from 'chart.js'
import { Line } from 'react-chartjs-2'

import type {
  AnalysisCurrentUserPressureDistribution,
  AnalysisCurrentUserPressureRow,
  AnalysisPressureMovingAverageKey,
  AnalysisPressureSnapshot,
} from '../api'
import type { AdminTranslations, Language } from '../i18n'
import AdminLoadingRegion from '../components/AdminLoadingRegion'

ChartJS.register(CategoryScale, LinearScale, LineElement, PointElement, Filler, Tooltip, Legend)

type ActiveUserPressurePoint = {
  rank: number
  label: string
  pressure: number
  successCount: number
  failureCount: number
}

function readChartColorVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  if (value.length === 0) return fallback
  return value.startsWith('hsl(') || value.startsWith('rgb(') ? value : `hsl(${value})`
}

function withOpacity(color: string, opacity: number): string {
  if (color.startsWith('hsl(') && color.endsWith(')')) {
    const body = color
      .slice(4, -1)
      .split('/')
      .shift()
      ?.trim() ?? ''
    return `hsl(${body} / ${opacity})`
  }
  return color
}

function formatNumber(language: Language, value: number): string {
  return new Intl.NumberFormat(language === 'zh' ? 'zh-CN' : 'en-US').format(value)
}

function formatAxisTime(language: Language, timestamp: number): string {
  return new Intl.DateTimeFormat(language === 'zh' ? 'zh-CN' : 'en-US', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(new Date(timestamp * 1000))
}

function formatAxisHour(language: Language, timestamp: number): string {
  return new Intl.DateTimeFormat(language === 'zh' ? 'zh-CN' : 'en-US', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    hour12: false,
  }).format(new Date(timestamp * 1000))
}

function formatUserLabel(
  language: Language,
  row: AnalysisCurrentUserPressureRow,
  fallbackLabel: string,
): string {
  const primary = row.displayName?.trim() || row.username?.trim() || fallbackLabel
  if (!row.username?.trim() || row.username === row.displayName) return primary
  return language === 'zh'
    ? `${primary}（@${row.username}）`
    : `${primary} (@${row.username})`
}

function buildActiveUserPressureCurve(
  distribution: AnalysisCurrentUserPressureDistribution,
  language: Language,
  fallbackLabel: string,
): ActiveUserPressurePoint[] {
  return distribution.rows
    .filter((row) => row.pressure > 0)
    .sort((left, right) => right.pressure - left.pressure || left.userId.localeCompare(right.userId))
    .map((row, index) => ({
      rank: index + 1,
      label: formatUserLabel(language, row, fallbackLabel),
      pressure: row.pressure,
      successCount: row.successCount,
      failureCount: row.failureCount,
    }))
}

function averagePressure(values: number[]): number {
  if (values.length === 0) return 0
  return values.reduce((sum, value) => sum + value, 0) / values.length
}

function buildPressureLineOptions(
  language: Language,
  tooltipLabelFormatter: (value: number) => string,
): ChartOptions<'line'> {
  const tickColor = readChartColorVar('--dashboard-chart-tick', '#635f69')
  const legendColor = readChartColorVar('--muted-foreground', '#635f69')
  const gridColor = readChartColorVar('--border', '#d7dfec')
  return {
    responsive: true,
    maintainAspectRatio: false,
    interaction: {
      mode: 'index',
      intersect: false,
    },
    plugins: {
      legend: {
        labels: {
          color: legendColor,
          boxWidth: 18,
          boxHeight: 8,
          usePointStyle: false,
        },
      },
      tooltip: {
        callbacks: {
          title(items) {
            const first = items[0]
            if (!first) return ''
            return tooltipLabelFormatter(Number(first.label))
          },
          label(context) {
            const value = typeof context.raw === 'number' ? context.raw : 0
            return `${context.dataset.label ?? ''}: ${formatNumber(language, value)}`
          },
        },
      },
    },
    scales: {
      x: {
        ticks: {
          color: tickColor,
          callback: function callback(value) {
            const label = typeof this.getLabelForValue === 'function'
              ? this.getLabelForValue(Number(value))
              : String(value)
            return tooltipLabelFormatter(Number(label))
          },
          maxRotation: 0,
          autoSkip: true,
          maxTicksLimit: 8,
        },
        grid: {
          color: withOpacity(gridColor, 0.34),
          drawTicks: false,
        },
      },
      y: {
        beginAtZero: true,
        ticks: {
          color: tickColor,
        },
        grid: {
          color: withOpacity(gridColor, 0.5),
          drawTicks: false,
        },
      },
    },
  }
}

function buildUserPressureCurveOptions(
  language: Language,
  rankLabel: string,
  points: ActiveUserPressurePoint[],
): ChartOptions<'line'> {
  const tickColor = readChartColorVar('--dashboard-chart-tick', '#635f69')
  const legendColor = readChartColorVar('--muted-foreground', '#635f69')
  const gridColor = readChartColorVar('--border', '#d7dfec')
  return {
    responsive: true,
    maintainAspectRatio: false,
    interaction: {
      mode: 'nearest',
      intersect: false,
    },
    plugins: {
      legend: {
        labels: {
          color: legendColor,
          boxWidth: 18,
          boxHeight: 8,
          usePointStyle: false,
        },
      },
      tooltip: {
        callbacks: {
          title(items) {
            const first = items[0]
            if (!first) return ''
            const point = points[first.dataIndex]
            return point?.label ?? ''
          },
          label(context) {
            const point = points[context.dataIndex]
            if (!point) return ''
            const successLabel = language === 'zh' ? '成功' : 'Success'
            const failureLabel = language === 'zh' ? '失败' : 'Failure'
            const rankText = `${rankLabel}: ${formatNumber(language, point.rank)}`
            const pressureText = `${context.dataset.label ?? ''}: ${formatNumber(language, point.pressure)}`
            const successText = `${successLabel}: ${formatNumber(language, point.successCount)}`
            const failureText = `${failureLabel}: ${formatNumber(language, point.failureCount)}`
            return [rankText, pressureText, successText, failureText]
          },
        },
      },
    },
    scales: {
      x: {
        title: {
          display: true,
          text: rankLabel,
          color: legendColor,
        },
        ticks: {
          color: tickColor,
          maxRotation: 0,
          autoSkip: true,
          maxTicksLimit: 10,
        },
        grid: {
          color: withOpacity(gridColor, 0.34),
          drawTicks: false,
        },
      },
      y: {
        beginAtZero: true,
        ticks: {
          color: tickColor,
        },
        grid: {
          color: withOpacity(gridColor, 0.5),
          drawTicks: false,
        },
      },
    },
  }
}

function movingAverageColor(key: AnalysisPressureMovingAverageKey, fallback: string): string {
  switch (key) {
    case 'sma6h':
      return readChartColorVar('--warning', fallback)
    case 'sma24h':
      return readChartColorVar('--success', fallback)
    default:
      return fallback
  }
}

function pointRadius(context: ScriptableContext<'line'>): number {
  const pointCount = context.chart.data.labels?.length ?? 0
  return pointCount > 72 ? 0 : 2.5
}

export interface PressureAnalysisScreenProps {
  snapshot: AnalysisPressureSnapshot | null
  loading: boolean
  error: string | null
  language: Language
  strings: AdminTranslations['pressure']
  onRetry: () => void
}

export default function PressureAnalysisScreen({
  snapshot,
  loading,
  error,
  language,
  strings,
  onRetry,
}: PressureAnalysisScreenProps): JSX.Element {
  const currentColor = readChartColorVar('--primary', '#7c3aed')
  const previousColor = readChartColorVar('--secondary', '#db2777')
  const hourlyColor = readChartColorVar('--info', '#0ea5e9')
  const averageColor = readChartColorVar('--muted-foreground', '#635f69')

  const current24hLabels = useMemo(
    () => snapshot?.server24h.current.map((point) => String(point.displayBucketStart)) ?? [],
    [snapshot],
  )
  const current24hAverage = useMemo(
    () => averagePressure(snapshot?.server24h.current.map((point) => point.pressure) ?? []),
    [snapshot],
  )
  const current24hData = useMemo<ChartData<'line'>>(() => ({
    labels: current24hLabels,
    datasets: [
      {
        label: strings.charts.last24h.currentLabel,
        data: snapshot?.server24h.current.map((point) => point.pressure) ?? [],
        borderColor: currentColor,
        backgroundColor: withOpacity(currentColor, 0.1),
        pointRadius,
        pointHoverRadius: 4,
        cubicInterpolationMode: 'monotone',
        tension: 0.32,
        borderWidth: 2.6,
      },
      {
        label: strings.charts.last24h.previousLabel,
        data: snapshot?.server24h.previous.map((point) => point.pressure) ?? [],
        borderColor: previousColor,
        backgroundColor: withOpacity(previousColor, 0.08),
        pointRadius: 0,
        cubicInterpolationMode: 'monotone',
        tension: 0.3,
        borderWidth: 1.85,
        borderDash: [6, 6],
      },
      {
        label: `${strings.charts.last24h.averageLabel} (${formatNumber(language, Math.round(current24hAverage * 10) / 10)})`,
        data: current24hLabels.map(() => current24hAverage),
        borderColor: averageColor,
        backgroundColor: withOpacity(averageColor, 0.04),
        pointRadius: 0,
        cubicInterpolationMode: 'monotone',
        tension: 0,
        borderWidth: 1.75,
        borderDash: [4, 5],
      },
    ],
  }), [
    averageColor,
    current24hAverage,
    current24hLabels,
    currentColor,
    language,
    previousColor,
    snapshot,
    strings.charts.last24h.averageLabel,
    strings.charts.last24h.currentLabel,
    strings.charts.last24h.previousLabel,
  ])

  const server7dLabels = useMemo(
    () => snapshot?.server7d.points.map((point) => String(point.displayBucketStart)) ?? [],
    [snapshot],
  )
  const server7dData = useMemo<ChartData<'line'>>(() => {
    const movingAverageLabels = new Map<AnalysisPressureMovingAverageKey, string>([
      ['sma6h', strings.charts.last7d.sma6hLabel],
      ['sma24h', strings.charts.last7d.sma24hLabel],
    ])
    const movingAverageDatasets = (snapshot?.server7d.movingAverages ?? []).map((series) => ({
      label: movingAverageLabels.get(series.key) ?? series.key,
      data: series.points.map((point) => point.value),
      borderColor: movingAverageColor(series.key, hourlyColor),
      backgroundColor: withOpacity(movingAverageColor(series.key, hourlyColor), 0.08),
      pointRadius: 0,
      cubicInterpolationMode: 'monotone' as const,
      tension: 0.26,
      borderWidth: 1.8,
      borderDash: [5, 5],
    }))
    return {
      labels: server7dLabels,
      datasets: [
        {
          label: strings.charts.last7d.seriesLabel,
          data: snapshot?.server7d.points.map((point) => point.pressure) ?? [],
          borderColor: hourlyColor,
          backgroundColor: withOpacity(hourlyColor, 0.1),
          pointRadius,
          pointHoverRadius: 4,
          cubicInterpolationMode: 'monotone',
          tension: 0.28,
          borderWidth: 2.25,
        },
        ...movingAverageDatasets,
      ],
    }
  }, [
    hourlyColor,
    server7dLabels,
    snapshot,
    strings.charts.last7d.seriesLabel,
    strings.charts.last7d.sma24hLabel,
    strings.charts.last7d.sma6hLabel,
  ])

  const userCurvePoints = useMemo(
    () => buildActiveUserPressureCurve(
      snapshot?.currentUserDistribution ?? {
        windowMinutes: 60,
        rows: [],
        summary: {
          activeUsers: 0,
          zeroPressureUsers: 0,
          median: 0,
          p90: 0,
          peak: 0,
          currentPressure: 0,
          vsYesterdayDelta: 0,
        },
      },
      language,
      strings.userFallback,
    ),
    [language, snapshot, strings.userFallback],
  )
  const userCurveData = useMemo<ChartData<'line'>>(() => ({
    labels: userCurvePoints.map((point) => String(point.rank)),
    datasets: [
      {
        label: strings.charts.userDistribution.seriesLabel,
        data: userCurvePoints.map((point) => point.pressure),
        borderColor: currentColor,
        backgroundColor: withOpacity(currentColor, 0.12),
        pointRadius: 2.75,
        pointHoverRadius: 4.5,
        cubicInterpolationMode: 'monotone',
        tension: 0.24,
        borderWidth: 2.35,
        fill: true,
      },
    ],
  }), [currentColor, strings.charts.userDistribution.seriesLabel, userCurvePoints])

  if (loading && !snapshot) {
    return (
      <AdminLoadingRegion
        loadState="initial_loading"
        loadingLabel={strings.loading}
        minHeight={420}
      />
    )
  }

  if (error && !snapshot) {
    return (
      <section className="surface panel pressure-analysis-empty-state" role="alert">
        <h2>{strings.errorTitle}</h2>
        <p className="panel-description">{error}</p>
        <button type="button" className="btn btn-outline" onClick={onRetry}>
          {strings.retry}
        </button>
      </section>
    )
  }

  if (!snapshot) {
    return (
      <section className="surface panel pressure-analysis-empty-state">
        <h2>{strings.emptyTitle}</h2>
        <p className="panel-description">{strings.emptyDescription}</p>
      </section>
    )
  }

  return (
    <div className="pressure-analysis-page" data-testid="pressure-analysis-screen">
      <section className="surface panel">
        <div className="panel-header">
          <div>
            <h2>{strings.charts.last24h.title}</h2>
            <p className="panel-description">{strings.charts.last24h.description}</p>
          </div>
        </div>
        <div className="pressure-chart-shell pressure-chart-shell-line">
          <Line
            data={current24hData}
            options={buildPressureLineOptions(language, (value) => formatAxisTime(language, value))}
          />
        </div>
      </section>

      <section className="surface panel">
        <div className="panel-header">
          <div>
            <h2>{strings.charts.userDistribution.title}</h2>
            <p className="panel-description">{strings.charts.userDistribution.description}</p>
          </div>
        </div>
        {userCurvePoints.length === 0 ? (
          <div className="empty-state alert">{strings.charts.userDistribution.empty}</div>
        ) : (
          <div
            className="pressure-chart-shell pressure-chart-shell-distribution"
            data-testid="pressure-distribution-histogram"
          >
            <Line
              data={userCurveData}
              options={buildUserPressureCurveOptions(
                language,
                strings.charts.userDistribution.rankLabel,
                userCurvePoints,
              )}
            />
          </div>
        )}
      </section>

      <section className="surface panel">
        <div className="panel-header">
          <div>
            <h2>{strings.charts.last7d.title}</h2>
            <p className="panel-description">{strings.charts.last7d.description}</p>
          </div>
        </div>
        <div className="pressure-chart-shell pressure-chart-shell-line">
          <Line
            data={server7dData}
            options={buildPressureLineOptions(language, (value) => formatAxisHour(language, value))}
          />
        </div>
      </section>
    </div>
  )
}
