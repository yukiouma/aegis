import { createTheme } from '@mui/material/styles';

export const totoroTheme = createTheme({
  palette: {
    primary: {
      main: '#4A7856', // Deep Forest Green
      light: '#76A681',
      dark: '#2D4B35',
      contrastText: '#FFFFFF',
    },
    secondary: {
      main: '#87CEEB', // Sky Blue
      light: '#B0E2FF',
    },
    background: {
      default: '#FFFDF5', // Soft Cream (Old Paper/Sketchbook vibe)
      paper: '#FFFFFF',
    },
    text: {
      primary: '#3E2723', // Dark Earthy Brown (instead of black)
      secondary: '#5D4037',
    },
    warning: {
      main: '#FFB74D', // Soft Acorn/Sunlight Orange
    },
  },
  shape: {
    borderRadius: 24, // Very rounded, organic corners
  },
  // typography: {
  //   // Friendly, rounded, and legible fonts
  //   fontFamily: '"Quicksand", "Nunito", sans-serif',
  //   h1: { fontWeight: 700, color: '#4A7856' },
  //   h2: { fontWeight: 700, color: '#4A7856' },
  //   button: {
  //     textTransform: 'none', // Keeps it casual and friendly
  //     fontWeight: 600,
  //   },
  // },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          padding: '8px 24px',
          boxShadow: 'none',
          '&:hover': {
            boxShadow: '0 4px 12px rgba(74, 120, 86, 0.2)',
            transform: 'translateY(-1px)',
          },
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          border: '2px solid #E0DBCB', // Soft sketchbook border
          boxShadow: '0 8px 24px rgba(62, 39, 35, 0.05)',
        },
      },
    },
    MuiCard: {
      styleOverrides: {
        root: {
          backgroundColor: '#FCF9F0',
        },
      },
    },
    MuiCssBaseline: {
      styleOverrides: `
        body {
          background-image: radial-gradient(#E0DBCB 1px, transparent 1px);
          background-size: 40px 40px; /* Subtle "Grid Paper" texture */
        }
      `,
    },
  },
});
