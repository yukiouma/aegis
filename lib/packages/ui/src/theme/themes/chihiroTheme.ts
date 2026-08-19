import { createTheme } from '@mui/material/styles';

export const chihiroTheme = createTheme({
  palette: {
    mode: 'light',
    primary: {
      main: '#C24B2B', // Sunset Vermilion (The color of the bridge at dusk)
      contrastText: '#FFFBF2',
    },
    secondary: {
      main: '#B8860B', // Dark Goldenrod (Old lantern brass)
    },
    background: {
      default: '#FDF5E6', // Old Lace / Aged Washi Paper
      paper: '#FFFBF2',   // Rich Cream
    },
    text: {
      primary: '#2D2926', // Charred Wood (Warm charcoal)
      secondary: '#8B4513', // Saddle Brown
    },
    divider: 'rgba(184, 134, 11, 0.3)',
  },
  shape: {
    borderRadius: 4, // Slightly thicker than before, like heavy cardstock
  },
  // typography: {
  //   fontFamily: '"EB Garamond", "Inter", serif',
  //   h1: {
  //     fontWeight: 800,
  //     color: '#C24B2B',
  //     fontFamily: '"EB Garamond", serif',
  //     letterSpacing: '-0.01em',
  //   },
  //   button: {
  //     fontWeight: 700,
  //     letterSpacing: '0.05em',
  //     textTransform: 'uppercase',
  //   },
  // },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          transition: 'all 0.3s ease',
          '&:hover': {
            boxShadow: '4px 4px 0px #E6AF2E', // Lantern Gold flat shadow
            transform: 'translate(-2px, -2px)',
          },
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: 'none',
          backgroundColor: '#FFFBF2',
          border: '1px solid #E6AF2E', // Subtle gold trim
          boxShadow: '8px 8px 0px rgba(184, 134, 11, 0.1)', // Structural "printed" shadow
          position: 'relative',
        },
      },
    },
    MuiAppBar: {
      styleOverrides: {
        root: {
          backgroundColor: '#2D2926', // Dark contrast like the bathhouse night sky
          color: '#E6AF2E',
          borderBottom: '3px solid #C24B2B',
        },
      },
    },
    MuiCssBaseline: {
      styleOverrides: `
        body {
          /* Adds a very subtle "Golden Mist" texture */
          background-image: radial-gradient(#E6AF2E 0.5px, transparent 0.5px);
          background-size: 30px 30px;
          background-color: #FDF5E6;
        }
      `,
    },
  },
});
