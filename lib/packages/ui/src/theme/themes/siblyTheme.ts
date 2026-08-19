import { createTheme } from '@mui/material/styles';

export const siblyTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#00D7FF', // The iconic Sibyl/Dominator Cyan
      light: '#70EFFF',
      dark: '#008EAB',
      contrastText: '#000000',
    },
    error: {
      main: '#FF3131', // High Crime Coefficient / Lethal Eliminator Red
    },
    background: {
      default: '#000000', // Pure black for high-contrast HUD feel
      paper: 'rgba(0, 20, 30, 0.6)', // Semi-transparent holographic panels
    },
    text: {
      primary: '#cdeef3',
      secondary: 'rgba(0, 215, 255, 0.7)',
    },
    divider: 'rgba(0, 215, 255, 0.3)',
  },
  shape: {
    borderRadius: 2, // Sharp edges for technical readouts
  },
  // typography: {
  //   fontFamily: '"Orbitron", "Roboto Mono", monospace',
  //   h1: { fontWeight: 900, letterSpacing: '0.2em' },
  //   button: { textTransform: 'uppercase', fontWeight: 700, letterSpacing: '0.1em' },
  // },
  components: {
    MuiCssBaseline: {
      styleOverrides: {
        body: {
          // Global "Scanline" and flickering screen effect
          backgroundImage: `linear-gradient(rgba(18, 16, 16, 0) 50%, rgba(0, 0, 0, 0.1) 50%),
                            linear-gradient(90deg, rgba(0, 215, 255, 0.03), rgba(0, 0, 0, 0), rgba(0, 215, 255, 0.03))`,
          backgroundSize: '100% 3px, 3px 100%',
          backgroundColor: '#000000',
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
          border: '1px solid rgba(0, 215, 255, 0.5)',
          backdropFilter: 'blur(8px)',
          boxShadow: '0 0 15px rgba(0, 215, 255, 0.15)',
        },
      },
    },
    MuiTypography: {
      styleOverrides: {
        root: {
          // Applying the "Glow" to all text
          textShadow: '0 0 8px rgba(0, 215, 255, 0.6)',
        },
      },
    },
    MuiButton: {
      styleOverrides: {
        root: {
          borderWidth: '2px',
          '&:hover': {
            borderWidth: '2px',
            backgroundColor: 'rgba(0, 215, 255, 0.1)',
            boxShadow: '0 0 20px rgba(0, 215, 255, 0.4)',
          },
        },
      },
    },
  },
});
