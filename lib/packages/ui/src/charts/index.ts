// Thin re-export of `@mui/x-charts@9.16.0`. Consumers compose the chart
// primitives directly; the UI package ships no chart wrapper. The
// dependency is a regular (not peer) dependency so the charts resolve
// the same `@mui/material` instance the rest of the app uses — a second
// instance would put the charts on a different theme context.
export * from '@mui/x-charts';
