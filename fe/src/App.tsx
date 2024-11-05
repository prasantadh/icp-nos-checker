import { useEffect, useState } from 'react'
import { Card, Text, Title, Grid, Modal } from '@mantine/core'
import { useDisclosure } from '@mantine/hooks';

interface Assignment {
  name: string,
  status: string,
}

interface Report {
  id: number,
  assignments: Assignment[],
}

function App() {

  const [data, setData] = useState<Report[]>([]);

  useEffect(() => {
    console.log(import.meta.env);
    console.log(import.meta.env.VITE_API_HOST);
    fetch(`${import.meta.env.VITE_API_HOST}`).then(
      (response) => {
        return response.json();
      }
    ).then((response: Report[]) => setData(response));
  }, []);

  const [opened, { open, close }] = useDisclosure(false);

  return (
    <>
      <Grid gutter={10}>
        {
          data.map((report) =>
            <Grid.Col span={{ base: 12, sm: 4 }} key={report.id}>
              <Card shadow="sm" padding="lg" radius="md" withBorder>
                <Title order={4} onClick={open}>ID: {report.id}</Title>
                {report.assignments.map(
                  (assignment, index) => (
                    <Text key={index}>
                      {assignment.name}: {assignment.status}
                    </Text>
                  )
                )}
              </Card>
            </Grid.Col >
          )
        }
      </Grid >
      <Modal opened={opened} onClose={close} title="Tree" centered>
        Submission Tree
      </Modal>
    </>
  )
}

export default App
